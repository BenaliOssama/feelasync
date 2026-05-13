use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::reactor::Reactor;
use crate::task::{Poll, Task};
use crate::waker::{ReadyQueue, TaskId, Waker};

/// A handle that can be cloned and given to tasks, letting them
/// spawn new tasks into the executor they live in.
#[derive(Clone)]
pub struct Spawner {
    pending: Rc<RefCell<Vec<Rc<RefCell<dyn Task>>>>>,
}

impl Spawner {
    pub fn spawn(&self, task: Rc<RefCell<dyn Task>>) {
        self.pending.borrow_mut().push(task);
    }
}

pub struct Executor {
    tasks: HashMap<TaskId, Rc<RefCell<dyn Task>>>,
    ready: ReadyQueue,
    next_id: TaskId,
    reactor: Rc<Reactor>,
    pending: Rc<RefCell<Vec<Rc<RefCell<dyn Task>>>>>,
}

impl Executor {
    pub fn new(reactor: Rc<Reactor>) -> Self {
        Self {
            tasks: HashMap::new(),
            ready: Rc::new(RefCell::new(Vec::new())),
            next_id: 0,
            reactor,
            pending: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn spawner(&self) -> Spawner {
        Spawner { pending: self.pending.clone() }
    }

    pub fn spawn(&mut self, task: Rc<RefCell<dyn Task>>) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.insert(id, task);
        self.ready.borrow_mut().push(id);
        id
    }

    /// Move anything in the pending-spawn buffer into the live task set.
    fn drain_pending(&mut self) {
        let new_tasks: Vec<_> = self.pending.borrow_mut().drain(..).collect();
        for t in new_tasks {
            self.spawn(t);
        }
    }

    pub fn run(&mut self) {
        loop {
            // Pull in anything tasks spawned during the previous poll.
            self.drain_pending();

            // Drain everything currently ready.
            loop {
                let id = match self.ready.borrow_mut().pop() {
                    Some(id) => id,
                    None => break,
                };

                let waker = Waker::new(id, self.ready.clone());
                let task = match self.tasks.get(&id) {
                    Some(t) => t.clone(),
                    None => continue,
                };

                let result = task.borrow_mut().poll(&waker);
                if matches!(result, Poll::Ready) {
                    self.tasks.remove(&id);
                }

                // A poll may have spawned new tasks — fold them in.
                self.drain_pending();
            }

            if self.tasks.is_empty() {
                break;
            }

            self.reactor.wait();
        }
    }
}
