//! A Waker is a small handle that, when called, marks a task as ready
//! to be polled again. It is what reactors call when their event fires.

use std::cell::RefCell;
use std::rc::Rc;

pub type TaskId = usize;
pub type ReadyQueue = Rc<RefCell<Vec<TaskId>>>;

#[derive(Clone)]
pub struct Waker {
    task_id: TaskId,
    queue: ReadyQueue,
}

impl Waker {
    pub fn new(task_id: TaskId, queue: ReadyQueue) -> Self {
        Self { task_id, queue }
    }

    pub fn wake(&self) {
        self.queue.borrow_mut().push(self.task_id);
    }
}
