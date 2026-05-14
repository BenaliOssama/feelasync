//! Bridge between our runtime and Rust's standard Waker.
//!
//! Rust's std::task::Waker is built on a vtable of 4 raw functions
//! (clone, wake, wake_by_ref, drop). We implement those 4 functions
//! so our wakers slot into Rust's async ecosystem and `async fn` works.
//!
//! Treat this as plumbing. The interesting part of the runtime is elsewhere.

use std::cell::RefCell;
use std::rc::Rc;
use std::task::{RawWaker, RawWakerVTable, Waker};

pub type TaskId = usize;
pub type ReadyQueue = Rc<RefCell<Vec<TaskId>>>;

/// What every waker carries: which task to wake, and where to drop the wake signal.
struct WakerData {
    task_id: TaskId,
    queue: ReadyQueue,
}

/// Build a std::task::Waker that, when called, pushes our task id onto the ready queue.
pub fn make_waker(task_id: TaskId, queue: ReadyQueue) -> Waker {
    // Box the data so it has a stable heap address; hand Rust a raw pointer to it.
    let data = Box::new(WakerData { task_id, queue });
    let raw = RawWaker::new(Box::into_raw(data) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw) }
}

// The 4 functions Rust calls on our waker. They cast the raw pointer
// back into our WakerData and do the right thing.
const VTABLE: RawWakerVTable = RawWakerVTable::new(clone_fn, wake_fn, wake_by_ref_fn, drop_fn);

/// clone: duplicate the waker (e.g. when something wants to keep a copy).
unsafe fn clone_fn(ptr: *const ()) -> RawWaker {
    let data = &*(ptr as *const WakerData);
    let cloned = Box::new(WakerData {
        task_id: data.task_id,
        queue: data.queue.clone(),
    });
    RawWaker::new(Box::into_raw(cloned) as *const (), &VTABLE)
}

/// wake: trigger the wake, then free the waker (it's consumed).
unsafe fn wake_fn(ptr: *const ()) {
    let data = Box::from_raw(ptr as *mut WakerData);
    data.queue.borrow_mut().push(data.task_id);
    // data drops here — memory freed.
}

/// wake_by_ref: trigger the wake but don't consume the waker.
unsafe fn wake_by_ref_fn(ptr: *const ()) {
    let data = &*(ptr as *const WakerData);
    data.queue.borrow_mut().push(data.task_id);
}

/// drop: free the waker when nobody holds it anymore.
unsafe fn drop_fn(ptr: *const ()) {
    drop(Box::from_raw(ptr as *mut WakerData));
}
