mod executor;
mod reactor;
mod task;
mod timer;
mod waker;

use std::cell::RefCell;
use std::io::{ErrorKind, Read};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, RawFd};
use std::rc::Rc;
use reactor::Reactor;
use task::{Poll, Task};
use timer::TimerTask;
use waker::Waker;

use executor::{Executor, Spawner};

struct AcceptTask {
    listener: TcpListener,
    spawner: Spawner,
    reactor: Rc<Reactor>,
    registered: bool,
}

impl Task for AcceptTask {
    fn poll(&mut self, waker: &Waker) -> Poll {
        if !self.registered {
            self.reactor.register(self.listener.as_raw_fd(), waker.clone());
            self.registered = true;
        }
        loop {
            match self.listener.accept() {
                Ok((socket, addr)) => {
                    socket.set_nonblocking(true).unwrap();
                    let sock_fd = socket.as_raw_fd();
                    println!("new conn fd {} from {}", sock_fd, addr);
                    self.spawner.spawn(Rc::new(RefCell::new(EchoTask {
                        socket,
                        fd: sock_fd,
                        reactor: self.reactor.clone(),
                        registered: false,
                    })));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Poll::Pending,
                Err(e) => {
                    eprintln!("accept error: {}", e);
                    return Poll::Pending;
                }
            }
        }
    }
}


struct EchoTask {
    socket: TcpStream,
    fd: RawFd,
    reactor: Rc<Reactor>,
    registered: bool,
}

impl Task for EchoTask {
    fn poll(&mut self, waker: &Waker) -> Poll {
        if !self.registered {
            self.reactor.register(self.fd, waker.clone());
            self.registered = true;
        }
        let mut buf = [0u8; 1024];
        loop {
            match self.socket.read(&mut buf) {
                Ok(0) => {
                    println!("fd {} closed", self.fd);
                    self.reactor.unregister(self.fd);
                    return Poll::Ready;
                }
                Ok(n) => println!("fd {} read {} bytes", self.fd, n),
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Poll::Pending,
                Err(e) => {
                    eprintln!("fd {} error: {}", self.fd, e);
                    self.reactor.unregister(self.fd);
                    return Poll::Ready;
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

    executor.spawn(Rc::new(RefCell::new(AcceptTask {
        listener,
        reactor: reactor.clone(),
        spawner: spawner.clone(),
        registered: false,
    })));

    executor.spawn(Rc::new(RefCell::new(TimerTask::new(3, "three-sec", reactor.clone()))));
    executor.spawn(Rc::new(RefCell::new(TimerTask::new(7, "seven-sec", reactor.clone()))));

    executor.run();   // <-- single call, runs until all tasks finish
}
