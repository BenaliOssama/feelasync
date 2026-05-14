//! A future that completes after a delay.

use std::future::Future;
use std::os::fd::RawFd;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use crate::reactor::Reactor;

pub struct TimerTask {
    pub fd: RawFd,
    pub name: &'static str,
    pub reactor: Rc<Reactor>,
    pub registered: bool,
}

impl TimerTask {
    pub fn new(seconds: i64, name: &'static str, reactor: Rc<Reactor>) -> Self {
        let fd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, 0) };
        assert!(fd >= 0);

        let spec = libc::itimerspec {
            it_interval: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            it_value:    libc::timespec { tv_sec: seconds, tv_nsec: 0 },
        };
        let r = unsafe { libc::timerfd_settime(fd, 0, &spec, std::ptr::null_mut()) };
        assert_eq!(r, 0);

        Self { fd, name, reactor, registered: false }
    }
}

impl Future for TimerTask {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if !self.registered {
            // Hand the reactor a clone of the current task's waker.
            self.reactor.register(self.fd, cx.waker().clone());
            self.registered = true;
            return Poll::Pending;
        }

        // Drain the 8-byte expiration count from the timerfd.
        let mut buf = [0u8; 8];
        let _ = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut _, 8) };

        println!(">>> timer [{}] fired", self.name);

        self.reactor.unregister(self.fd);
        unsafe { libc::close(self.fd) };
        Poll::Ready(())
    }
}
