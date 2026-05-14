//! A future that completes when an fd becomes readable.
//! Tasks use this to "await" socket readiness.

use std::future::Future;
use std::os::fd::RawFd;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use crate::reactor::Reactor;

pub struct Readable {
    pub fd: RawFd,
    pub reactor: Rc<Reactor>,
    pub registered: bool,
}

impl Future for Readable {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if !self.registered {
            self.reactor.register(self.fd, cx.waker().clone());
            self.registered = true;
            return Poll::Pending;
        }
        // We were woken — fd is ready.
        Poll::Ready(())
    }
}

/// Returns a future that resolves when `fd` is readable.
pub fn readable(fd: RawFd, reactor: Rc<Reactor>) -> Readable {
    Readable { fd, reactor, registered: false }
}
