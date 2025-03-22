use crossbeam_channel::Sender;
pub enum Event {
    End,
}

pub enum TaskEvent {
    Progress { id: i64, finished: i64, total: i64 },
    Cancel { id: i64 },
    End { id: i64 },
}

pub struct Task {
    id: i64,
    tx: Sender<TaskEvent>,
}

impl Drop for Task {
    fn drop(&mut self) {
        let _ = self.tx.send(TaskEvent::End { id: self.id });
    }
}
