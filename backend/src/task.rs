use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{atomic::AtomicI64, Arc};
use std::thread;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

enum TaskOneshotEvent {
    Cancel,
}

struct Task {
    id: i64,
    handle: JoinHandle<()>,
    sender: Option<oneshot::Sender<TaskOneshotEvent>>,
    name: Option<String>,
    // status: TaskStatus,
}

enum TaskManagerEvent {
    Add(Task),
    Progress,
    Cancel { id: i64 },
    End { id: i64 },
    Stop,
    // Wait,
    // Pause,
}
pub struct TaskOper {
    id: i64,
    tx: Sender<TaskManagerEvent>,
}

#[derive(Debug)]
struct TaskManagerInner {
    id: AtomicI64,
}

#[derive(Debug, Clone)]
pub struct TaskManagerOper {
    inner: Arc<TaskManagerInner>,
    sender: Sender<TaskManagerEvent>,
}

impl TaskManagerOper {
    fn gen_id(&self) -> i64 {
        return self
            .inner
            .id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn add(&self, task: Task) {
        let _ = self.sender.send(TaskManagerEvent::Add(task));
    }

    pub fn canel(&self, id: i64) {
        let _ = self.sender.send(TaskManagerEvent::Cancel { id });
    }

    pub fn stop(&self) {
        let _ = self.sender.send(TaskManagerEvent::Stop);
    }

    pub fn spawn<F>(&self, future: F) -> i64
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task_id = self.gen_id();
        let (tx, rx) = oneshot::channel();
        let mgr_tx = self.sender.clone();

        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = future => {},
                one = rx => match one {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }

            let _ = mgr_tx.send(TaskManagerEvent::End { id: task_id });
        });

        let task = Task {
            id: task_id,
            handle,
            sender: Some(tx),
            name: None,
        };
        self.add(task);
        task_id
    }
}

pub struct TaskManager {
    inner: Arc<TaskManagerInner>,
    receiver: Receiver<TaskManagerEvent>,
    tasks: HashMap<i64, Task>,
}

impl TaskManager {
    pub fn new() -> (TaskManagerOper, Self) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let inner = Arc::new(TaskManagerInner {
            id: AtomicI64::new(0),
        });
        (
            TaskManagerOper {
                inner: inner.clone(),
                sender: tx,
            },
            Self {
                inner: inner,
                receiver: rx,
                tasks: HashMap::new(),
            },
        )
    }

    pub fn start(mut self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            self.process();
        })
    }

    pub fn process(&mut self) {
        while let Ok(event) = self.receiver.recv() {
            match event {
                TaskManagerEvent::Add(task) => {
                    self.tasks.insert(task.id, task);
                }
                TaskManagerEvent::Cancel { id } => {
                    if let Some(t) = self.tasks.get_mut(&id) {
                        if let Some(tx) = t.sender.take() {
                            let _ = tx.send(TaskOneshotEvent::Cancel);
                        }
                    }
                }
                TaskManagerEvent::End { id } => {
                    self.tasks.remove(&id);
                }
                TaskManagerEvent::Progress => {}
                TaskManagerEvent::Stop => {
                    break;
                }
            }
        }
    }
}
