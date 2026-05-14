//! Owns the epoll. Maps fd → waker. When an fd is ready, calls its waker,
//! which pushes the task id onto the executor's ready queue.

use std::cell::RefCell;
use std::collections::HashMap;
use std::os::fd::RawFd;
use std::rc::Rc;
use std::task::Waker;

pub struct Reactor {
    epfd: i32,
    wakers: Rc<RefCell<HashMap<RawFd, Waker>>>,
}

impl Reactor {
    pub fn new() -> Self {
        let epfd = unsafe { libc::epoll_create1(0) };
        assert!(epfd >= 0);
        Self {
            epfd,
            wakers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn register(&self, fd: RawFd, waker: Waker) {
        let mut event = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: fd as u64,
        };
        let r = unsafe {
            libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_ADD, fd, &mut event)
        };
        if r != 0 {
            // Already registered → just update the waker.
            let r2 = unsafe {
                libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_MOD, fd, &mut event)
            };
            assert_eq!(r2, 0, "epoll_ctl MOD failed");
        }
        self.wakers.borrow_mut().insert(fd, waker);
    }

    pub fn unregister(&self, fd: RawFd) {
        let r = unsafe {
            libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut())
        };
        assert_eq!(r, 0);
        self.wakers.borrow_mut().remove(&fd);
    }

    /// Block on epoll. When events arrive, call the waker for each ready fd.
    /// That moves tasks onto the executor's ready queue.
    pub fn wait(&self) {
        let mut raw: [libc::epoll_event; 16] = unsafe { std::mem::zeroed() };
        let n = unsafe {
            libc::epoll_wait(self.epfd, raw.as_mut_ptr(), raw.len() as i32, -1)
        };
        assert!(n >= 0);

        for i in 0..n as usize {
            let fd = raw[i].u64 as RawFd;
            if let Some(waker) = self.wakers.borrow().get(&fd) {
                waker.wake_by_ref();
            }
        }
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        unsafe { libc::close(self.epfd) };
    }
}
