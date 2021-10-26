use std::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    ptr,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Mutex,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    thread::{self, JoinHandle},
};

use lazy_static::lazy_static;
use spin::Mutex as SpinMutex;

struct SyncChannel {
    sender: SyncSender<ExecutorMessage>,
    receiver: Mutex<Receiver<ExecutorMessage>>,
}

lazy_static! {
    static ref SYNC_CHANNEL: SyncChannel = {
        let (sender, receiver) = sync_channel(10);
        SyncChannel {
            sender,
            receiver: Mutex::new(receiver),
        }
    };
}

struct TaskPtr {
    inner: *mut Task,
}

/// SAFETY: TaskPtr is only dereferenced by the executor
unsafe impl Send for TaskPtr {}

struct Task {
    future: Pin<&'static mut (dyn Future<Output = ()>)>,
    join_handle: *const TaskJoinHandle,
}

impl Task {
    fn new(future: Pin<&mut (dyn Future<Output = ()>)>) -> Self {
        // SAFETY: run_batch() always joins futures before returning
        let future = unsafe {
            std::mem::transmute::<_, Pin<&'static mut (dyn Future<Output = ()>)>>(future)
        };

        Self {
            future,
            join_handle: ptr::null_mut(),
        }
    }

    fn run(mut self: Pin<&mut Self>, join_handle: &Pin<&mut TaskJoinHandle>) {
        // SAFETY: we ensure that join_handle lives as long as self
        self.join_handle =
            unsafe { std::mem::transmute::<_, *const TaskJoinHandle>(&**join_handle) };

        let message = ExecutorMessage::Task(TaskPtr { inner: &mut *self });

        SYNC_CHANNEL
            .sender
            .send(message)
            .expect("executor channel full");
    }

    fn poll_future(&mut self) {
        let waker = RawWaker::new(self as *mut Task as *mut (), &VTABLE);
        let waker = unsafe { Waker::from_raw(waker) };

        match self.future.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Ready(_) => {
                // SAFETY: technically breaking the rules by creating a reference while we could have
                // a &mut TaskJoinHandle, but we're only accessing Sync members of TaskJoinHandle
                let mut join_handle = unsafe { self.join_handle.as_ref().unwrap().inner.lock() };

                join_handle.done = true;

                if let Some(waker) = join_handle.waker.take() {
                    waker.wake();
                }
            }
            Poll::Pending => {}
        }
    }
}

fn task_clone(s: *mut Task) -> RawWaker {
    RawWaker::new(s as *const (), &VTABLE)
}

fn task_wake(task: *mut Task) {
    let message = ExecutorMessage::Task(TaskPtr { inner: task });
    SYNC_CHANNEL
        .sender
        .try_send(message)
        .expect("executor channel full");
}

const VTABLE: RawWakerVTable = {
    RawWakerVTable::new(
        |s| task_clone(s as *mut Task),
        |s| task_wake(s as *mut Task),
        |_| {},
        |_| {},
    )
};

struct TaskJoinHandle {
    inner: SpinMutex<TaskJoinHandleInner>,
}

struct TaskJoinHandleInner {
    done: bool,
    waker: Option<Waker>,
}

impl TaskJoinHandle {
    fn new() -> Self {
        let inner = TaskJoinHandleInner {
            done: false,
            waker: None,
        };

        TaskJoinHandle {
            inner: SpinMutex::new(inner),
        }
    }
}

impl Future for TaskJoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.inner.lock();

        if inner.done {
            Poll::Ready(())
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct Executor {
    thread_join_handles: Vec<JoinHandle<()>>,
}

enum ExecutorMessage {
    Task(TaskPtr),
    Join,
}

impl Executor {
    pub fn new() -> Self {
        const CPU_COUNT: usize = 4;

        let mut thread_join_handles = Vec::with_capacity(CPU_COUNT);

        for _ in 0..CPU_COUNT {
            thread_join_handles.push(thread::spawn(|| {
                loop {
                    match SYNC_CHANNEL.receiver.lock().unwrap().recv().unwrap() {
                        ExecutorMessage::Task(task_ptr) => {
                            // SAFETY: we only ever create a Task reference here
                            let task = unsafe { task_ptr.inner.as_mut().unwrap() };
                            task.poll_future();
                        }
                        ExecutorMessage::Join => break,
                    }
                }
            }));
        }

        Self {
            thread_join_handles,
        }
    }

    pub fn execute_blocking<F: Future<Output = ()>>(&self, future: Pin<&mut F>) {
        let mut task = Task::new(future);
        let mut task = unsafe { Pin::new_unchecked(&mut task) };

        let mut task_join = TaskJoinHandle::new();
        let task_join = unsafe { Pin::new_unchecked(&mut task_join) };

        task.join_handle = unsafe { std::mem::transmute::<_, *const TaskJoinHandle>(&*task_join) };
        task.poll_future();

        while !task_join.inner.lock().done {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        for _ in &self.thread_join_handles {
            SYNC_CHANNEL
                .sender
                .try_send(ExecutorMessage::Join)
                .expect("executor channel full");
        }

        for thread in self.thread_join_handles.drain(..) {
            thread.join().unwrap();
        }
    }
}

macro_rules! pin_array {
    ($arr: ident, $len: expr) => {
        let $arr = {
            unsafe {
                let mut x: [MaybeUninit<_>; $len] = MaybeUninit::uninit().assume_init();
                for (i, a) in $arr.iter_mut().enumerate() {
                    x[i].write(Pin::new_unchecked(a));
                }
                x.map(|a| a.assume_init())
            }
        };
    };
}

pub async fn run_batch<F: Fn(usize), const N: usize>(f: F) {
    let f = &f;
    let f_async = |index: usize| async move { f(index) };

    let mut futures = unsafe {
        let mut futures: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for (i, future) in futures.iter_mut().enumerate() {
            future.write(f_async(i));
        }
        futures.map(|a| a.assume_init())
    };
    pin_array!(futures, N);

    let mut tasks = unsafe {
        let mut tasks: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for (i, future) in futures.into_iter().enumerate() {
            tasks[i].write(Task::new(future));
        }
        tasks.map(|a| a.assume_init())
    };
    pin_array!(tasks, N);

    let mut join_handles = unsafe {
        let mut join_handles: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for i in 0..N {
            join_handles[i].write(TaskJoinHandle::new());
        }
        join_handles.map(|a| a.assume_init())
    };
    pin_array!(join_handles, N);

    for (i, task) in tasks.into_iter().enumerate() {
        task.run(&join_handles[i]);
    }

    for join_handle in join_handles {
        join_handle.await;
    }
}

pub async fn run_slice<T, F: Fn(&mut T)>(slice: &mut [T], f: F) {
    const CONCURRENCY: usize = 8;
    const STRIDE: usize = 8;

    let slice_ptr = slice.as_mut_ptr();
    let slice_len = slice.len();

    let f = &f;
    let f_async = |offset: usize| async move {
        if offset < slice_len {
            let len = usize::min(STRIDE, slice_len - offset);
            for i in 0..len {
                f(unsafe { &mut *slice_ptr.add(offset + i) });
            }
        }
    };

    let mut offset = 0;
    while offset < slice_len {
        let mut futures = unsafe {
            let mut futures: [MaybeUninit<_>; CONCURRENCY] = MaybeUninit::uninit().assume_init();
            for future in futures.iter_mut() {
                future.write(f_async(offset));
                offset += STRIDE;
            }
            futures.map(|a| a.assume_init())
        };
        pin_array!(futures, CONCURRENCY);

        let mut tasks = unsafe {
            let mut tasks: [MaybeUninit<_>; CONCURRENCY] = MaybeUninit::uninit().assume_init();
            for (i, future) in futures.into_iter().enumerate() {
                tasks[i].write(Task::new(future));
            }
            tasks.map(|a| a.assume_init())
        };
        pin_array!(tasks, CONCURRENCY);

        let mut join_handles = unsafe {
            let mut join_handles: [MaybeUninit<_>; CONCURRENCY] =
                MaybeUninit::uninit().assume_init();
            for i in 0..CONCURRENCY {
                join_handles[i].write(TaskJoinHandle::new());
            }
            join_handles.map(|a| a.assume_init())
        };
        pin_array!(join_handles, CONCURRENCY);

        for (i, task) in tasks.into_iter().enumerate() {
            task.run(&join_handles[i]);
        }

        for join_handle in join_handles {
            join_handle.await;
        }
    }
}
