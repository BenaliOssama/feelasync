//! A task that fires once after a delay, then completes.

use std::os::fd::RawFd;
use std::rc::Rc;
use crate::reactor::Reactor;
use crate::task::{Poll, Task};
use crate::waker::Waker;

pub struct TimerTask {
    pub fd: RawFd,
    pub name: &'static str,
    pub reactor: Rc<Reactor>,
    pub registered: bool,
}

impl TimerTask {
    pub fn new(seconds: i64, name: &'static str, reactor: Rc<Reactor>) -> Self {
        let fd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, 0) };
        assert!(fd >= 0, "timerfd_create failed");

        let spec = libc::itimerspec {
            it_interval: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            it_value:    libc::timespec { tv_sec: seconds, tv_nsec: 0 },
        };
        let r = unsafe { libc::timerfd_settime(fd, 0, &spec, std::ptr::null_mut()) };
        assert_eq!(r, 0);

        Self { fd, name, reactor, registered: false }
    }
}

impl Task for TimerTask {
    fn poll(&mut self, waker: &Waker) -> Poll {
        // First poll: register the timerfd with the reactor so the waker
        // gets called when the timer expires.
        if !self.registered {
            self.reactor.register(self.fd, waker.clone());
            self.registered = true;
            return Poll::Pending;
        }

        // We were woken — the timer fired. Drain the 8-byte expiration count.
        let mut buf = [0u8; 8];
        let _ = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut _, 8) };

        println!(">>> timer [{}] fired", self.name);

        self.reactor.unregister(self.fd);
        unsafe { libc::close(self.fd) };
        Poll::Ready
    }
}
