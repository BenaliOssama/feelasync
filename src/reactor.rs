//! Owns the epoll instance. Lets the rest of the program register fds
//! and wait for events without touching libc directly.

use std::os::fd::RawFd;

pub struct Reactor {
    epfd: i32,
}

pub struct Event {
    pub fd: RawFd,
}

impl Reactor {
    pub fn new() -> Self {
        let epfd = unsafe { libc::epoll_create1(0) };
        assert!(epfd >= 0, "epoll_create1 failed");
        Self { epfd }
    }


    /// Register an fd for read-readiness notifications.
    pub fn register(&self, fd: RawFd) {
        let mut event = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: fd as u64,
        };
        let r = unsafe {
            libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_ADD, fd, &mut event)
        };
        assert_eq!(r, 0, "epoll_ctl ADD failed");
    }

    pub fn unregister(&self, fd: RawFd) {
        let r = unsafe {
            libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut())
        };
        assert_eq!(r, 0, "epoll_ctl DEL failed");
    }

    /// Block until at least one fd is ready. Returns the list of ready fds.
    pub fn wait(&self) -> Vec<Event> {
        let mut raw: [libc::epoll_event; 16] = unsafe { std::mem::zeroed() };
        let n = unsafe {
            libc::epoll_wait(self.epfd, raw.as_mut_ptr(), raw.len() as i32, -1)
        };
        assert!(n >= 0, "epoll_wait failed");

        (0..n as usize)
            .map(|i| Event { fd: raw[i].u64 as RawFd })
            .collect()
    }
}


impl Drop for Reactor {
    fn drop(&mut self) {
        unsafe { libc::close(self.epfd) };
    }
}
