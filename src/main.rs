mod reactor;

use std::collections::HashMap;
use std::io::{ErrorKind, Read};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, RawFd};
use reactor::Reactor;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener_fd = listener.as_raw_fd();
    println!("listening on 127.0.0.1:7878");

    let reactor = Reactor::new();
    reactor.register(listener_fd);

    let mut connections: HashMap<RawFd, TcpStream> = HashMap::new();

    loop {
        let events = reactor.wait();

        for event in events {
            if event.fd == listener_fd {
                match listener.accept() {
                    Ok((socket, addr)) => {
                        socket.set_nonblocking(true).unwrap();
                        let sock_fd = socket.as_raw_fd();
                        println!("new conn fd {} from {}", sock_fd, addr);
                        reactor.register(sock_fd);
                        connections.insert(sock_fd, socket);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => eprintln!("accept error: {}", e),
                }
            } else if let Some(socket) = connections.get_mut(&event.fd) {
                let mut buf = [0u8; 1024];
                match socket.read(&mut buf) {
                    Ok(0) => {
                        println!("fd {} closed", event.fd);
                        reactor.unregister(event.fd);
                        connections.remove(&event.fd);
                    }
                    Ok(n) => {
                        println!("fd {} read {} bytes", event.fd, n);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => {
                        eprintln!("fd {} error: {}", event.fd, e);
                        reactor.unregister(event.fd);
                        connections.remove(&event.fd);
                    }
                }
            }
        }
    }
}
