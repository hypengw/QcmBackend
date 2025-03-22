use qcm_core::event::TaskEvent;
use std::sync::atomic::AtomicI64;

pub struct TaskManager {
    id: AtomicI64,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            id: AtomicI64::new(0),
        }
    }
}
