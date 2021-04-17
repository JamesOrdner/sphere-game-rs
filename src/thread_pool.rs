use crate::state_manager::Sender;
use core::ops::FnOnce;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

type Thunk<'a> = Box<dyn FnOnce(&mut Sender) + Send + 'a>;

enum Task {
    Thunk { thunk: Thunk<'static> },
    Join,
}

pub struct ThreadPool {
    threads: Vec<thread::JoinHandle<()>>,
    message_bus_senders: Vec<Sender>,
    task_sender: mpsc::SyncSender<Task>,
    num_pending_tasks: Arc<Mutex<usize>>,
    cvar: Arc<Condvar>,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        let mut threads = Vec::with_capacity(num_threads);
        let mut message_bus_senders = Vec::<Sender>::with_capacity(num_threads);

        let (task_sender, task_receiver) = mpsc::sync_channel(num_threads);
        let shared_receiver = Arc::new(Mutex::new(task_receiver));

        let num_pending_tasks = Arc::new(Mutex::new(0));
        let cvar = Arc::new(Condvar::new());

        for _ in 0..num_threads {
            message_bus_senders.push(Sender::new());
        }

        for i in 0..num_threads {
            let mut message_bus_sender =
                unsafe { message_bus_senders.as_mut_ptr().add(i).as_mut().unwrap() };

            let task_receiver = shared_receiver.clone();
            let num_pending_tasks = num_pending_tasks.clone();
            let cvar = cvar.clone();

            threads.push(thread::spawn(move || loop {
                match task_receiver.lock().unwrap().recv().unwrap() {
                    Task::Thunk { thunk } => {
                        thunk(&mut message_bus_sender);
                    }
                    Task::Join => break,
                }

                *num_pending_tasks.lock().unwrap() -= 1;
                cvar.notify_one();
            }));
        }

        ThreadPool {
            threads,
            message_bus_senders,
            task_sender,
            num_pending_tasks,
            cvar,
        }
    }

    pub fn scoped<'scope, F>(&'scope self, f: F)
    where
        F: FnOnce(Scope<'scope>),
    {
        let scope = Scope { thread_pool: self };
        f(scope);
    }

    pub fn get_message_bus_senders(&self) -> &[Sender] {
        self.message_bus_senders.as_slice()
    }

    pub fn get_message_bus_senders_mut(&mut self) -> &mut [Sender] {
        self.message_bus_senders.as_mut_slice()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in 0..self.threads.len() {
            self.task_sender.send(Task::Join).unwrap();
        }

        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }
    }
}

pub struct Scope<'scope> {
    thread_pool: &'scope ThreadPool,
}

impl<'scope> Scope<'scope> {
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce(&mut Sender) + Send + 'scope,
    {
        *self.thread_pool.num_pending_tasks.lock().unwrap() += 1;

        // safe: Scope::Drop() blocks until all tasks executed in 'scope are complete
        let thunk = unsafe { std::mem::transmute::<Thunk<'scope>, Thunk<'static>>(Box::new(f)) };
        self.thread_pool
            .task_sender
            .send(Task::Thunk { thunk })
            .unwrap();
    }
}

impl<'scope> Drop for Scope<'scope> {
    fn drop(&mut self) {
        let _ = self
            .thread_pool
            .cvar
            .wait_while(
                self.thread_pool.num_pending_tasks.lock().unwrap(),
                |num_pending_tasks| *num_pending_tasks > 0,
            )
            .unwrap();
    }
}
