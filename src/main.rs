use std::io::{ErrorKind, Read};
use std::net::TcpListener;
use std::os::fd::AsRawFd;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener_fd = listener.as_raw_fd();
    println!("listening on 127.0.0.1:7878 (fd {})", listener_fd);

    // 1. Create an epoll instance.
    let epfd = unsafe { libc::epoll_create1(0) };
    assert!(epfd >= 0, "epoll_create1 failed");

    // 2. Register the listener fd. EPOLLIN = "wake me when readable".
    //    For a listener, "readable" means "a new connection is waiting".
    let mut event = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: listener_fd as u64, // we get this back when it fires; use it as identity
    };
    let r = unsafe {
        libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, listener_fd, &mut event)
    };
    assert_eq!(r, 0, "epoll_ctl failed");

    // 3. Wait for events. Thread parks here (0% CPU) until something is ready.
    println!("parking on epoll_wait...");
    let mut events: [libc::epoll_event; 16] = unsafe { std::mem::zeroed() };
    let n = unsafe {
        libc::epoll_wait(epfd, events.as_mut_ptr(), events.len() as i32, -1)
    };
    assert!(n >= 0, "epoll_wait failed");
    println!("epoll_wait returned {} event(s)", n);

    // 4. Something is ready. We only registered the listener, so it must be that.
    let (mut socket, addr) = match listener.accept() {
        Ok(pair) => pair,
        Err(e) => panic!("accept failed: {}", e),
    };
    println!("got connection from {}", addr);

    // For this step we go back to blocking read on the socket — we're only
    // demonstrating epoll-based parking on accept. Step 4 will epoll the read too.
    socket.set_nonblocking(false).unwrap();
    let mut buf = [0u8; 1024];
    let n = socket.read(&mut buf).unwrap();
    println!("read {} bytes: {:?}", n, &buf[..n]);

    unsafe { libc::close(epfd) };
}
