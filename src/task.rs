use crate::waker::Waker;

pub enum Poll {
    Ready,
    Pending,
}

pub trait Task {
    fn poll(&mut self, waker: &Waker) -> Poll;
}
