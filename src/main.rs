use std::collections::HashMap;
use std::io::{ErrorKind, Read};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, RawFd};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener_fd = listener.as_raw_fd();
    println!("listening on 127.0.0.1:7878 (fd {})", listener_fd);

    let epfd = unsafe { libc::epoll_create1(0) };
    assert!(epfd >= 0);
    register(epfd, listener_fd);

    // Connection sockets, keyed by their fd.
    let mut connections: HashMap<RawFd, TcpStream> = HashMap::new();

    // Each connection has an idle-timeout timer fd.
    // Two maps to navigate the relationship in both directions:
    let mut conn_to_timer: HashMap<RawFd, RawFd> = HashMap::new();
    let mut timer_to_conn: HashMap<RawFd, RawFd> = HashMap::new();

    let mut events: [libc::epoll_event; 16] = unsafe { std::mem::zeroed() };

    loop {
        let n = unsafe {
            libc::epoll_wait(epfd, events.as_mut_ptr(), events.len() as i32, -1)
        };
        assert!(n >= 0);

        for i in 0..n as usize {
            let fd = events[i].u64 as RawFd;

            if fd == listener_fd {
                // ---- a new connection arrives ----
                match listener.accept() {
                    Ok((socket, addr)) => {
                        socket.set_nonblocking(true).unwrap();
                        let sock_fd = socket.as_raw_fd();
                        println!("new conn fd {} from {}", sock_fd, addr);
                        register(epfd, sock_fd);
                        connections.insert(sock_fd, socket);

                        // Create the per-connection idle timer.
                        let tfd = create_timer(10);
                        register(epfd, tfd);
                        conn_to_timer.insert(sock_fd, tfd);
                        timer_to_conn.insert(tfd, sock_fd);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => eprintln!("accept error: {}", e),
                }
            } else if let Some(&conn_fd) = timer_to_conn.get(&fd) {
                // ---- a per-connection idle timer fired ----
                let mut buf = [0u8; 8];
                let _ = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, 8) };
                println!("conn fd {} idle for 10s, closing", conn_fd);
                close_connection(epfd, conn_fd, &mut connections, &mut conn_to_timer, &mut timer_to_conn);
            } else if let Some(socket) = connections.get_mut(&fd) {
                // ---- a connection sent bytes ----
                let mut buf = [0u8; 1024];
                match socket.read(&mut buf) {
                    Ok(0) => {
                        println!("fd {} closed by peer", fd);
                        close_connection(epfd, fd, &mut connections, &mut conn_to_timer, &mut timer_to_conn);
                    }
                    Ok(read_n) => {
                        println!("fd {} read {} bytes", fd, read_n);
                        // Reset the idle timer.
                        if let Some(&tfd) = conn_to_timer.get(&fd) {
                            arm_timer(tfd, 10);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => {
                        eprintln!("fd {} read error: {}", fd, e);
                        close_connection(epfd, fd, &mut connections, &mut conn_to_timer, &mut timer_to_conn);
                    }
                }
            }
        }
    }
}

fn close_connection(
    epfd: i32,
    conn_fd: RawFd,
    connections: &mut HashMap<RawFd, TcpStream>,
    conn_to_timer: &mut HashMap<RawFd, RawFd>,
    timer_to_conn: &mut HashMap<RawFd, RawFd>,
) {
    unregister(epfd, conn_fd);
    connections.remove(&conn_fd);
    if let Some(tfd) = conn_to_timer.remove(&conn_fd) {
        unregister(epfd, tfd);
        timer_to_conn.remove(&tfd);
        unsafe { libc::close(tfd) };
    }
}

fn create_timer(seconds: i64) -> RawFd {
    let tfd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, 0) };
    assert!(tfd >= 0);
    arm_timer(tfd, seconds);
    tfd
}

fn arm_timer(tfd: RawFd, seconds: i64) {
    let spec = libc::itimerspec {
        it_interval: libc::timespec { tv_sec: 0, tv_nsec: 0 },
        it_value:    libc::timespec { tv_sec: seconds, tv_nsec: 0 },
    };
    unsafe { libc::timerfd_settime(tfd, 0, &spec, std::ptr::null_mut()) };
}

fn register(epfd: i32, fd: RawFd) {
    let mut event = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: fd as u64,
    };
    let r = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, fd, &mut event) };
    assert_eq!(r, 0);
}

fn unregister(epfd: i32, fd: RawFd) {
    let r = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };
    assert_eq!(r, 0);
}
