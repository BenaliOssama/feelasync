mod executor;
mod io;
mod reactor;
mod timer;
mod waker;

use std::io::{ErrorKind, Read};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, RawFd};
use std::rc::Rc;

use executor::{Executor, Spawner};
use reactor::Reactor;
use timer::TimerTask;
use crate::io::readable;

async fn handle_connection(mut socket: TcpStream, fd: RawFd, reactor: Rc<Reactor>) {
    let mut buf = [0u8; 1024];
    loop {
        readable(fd, reactor.clone()).await;

        loop {
            match socket.read(&mut buf) {
                Ok(0) => {
                    println!("fd {} closed", fd);
                    reactor.unregister(fd);
                    return;
                }
                Ok(n) => println!("fd {} read {} bytes", fd, n),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break, // back to await
                Err(e) => {
                    eprintln!("fd {} error: {}", fd, e);
                    reactor.unregister(fd);
                    return;
                }
            }
        }
    }
}

async fn accept_loop(listener: TcpListener, reactor: Rc<Reactor>, spawner: Spawner) {
    let listener_fd = listener.as_raw_fd();
    loop {
        readable(listener_fd, reactor.clone()).await;

        loop {
            match listener.accept() {
                Ok((socket, addr)) => {
                    socket.set_nonblocking(true).unwrap();
                    let sock_fd = socket.as_raw_fd();
                    println!("new conn fd {} from {}", sock_fd, addr);
                    spawner.spawn(handle_connection(socket, sock_fd, reactor.clone()));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    eprintln!("accept error: {}", e);
                    break;
                }
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();
    println!("listening on 127.0.0.1:7878");

    let reactor = Rc::new(Reactor::new());
    let mut executor = Executor::new(reactor.clone());
    let spawner = executor.spawner();

    executor.spawn(accept_loop(listener, reactor.clone(), spawner));
    executor.spawn(TimerTask::new(3, "three-sec", reactor.clone()));
    executor.spawn(TimerTask::new(7, "seven-sec", reactor.clone()));

    executor.run();
}
