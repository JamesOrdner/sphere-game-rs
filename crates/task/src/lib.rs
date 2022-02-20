use std::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    ptr,
    sync::{
        atomic::{AtomicPtr, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Condvar, Mutex,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    thread::{self, JoinHandle, ThreadId},
};

use lazy_static::lazy_static;
use spin::Mutex as SpinMutex;

struct SyncChannel {
    sender: SyncSender<ExecutorMessage>,
    receiver: Mutex<Receiver<ExecutorMessage>>,
}

lazy_static! {
    static ref SYNC_CHANNEL: SyncChannel = {
        let (sender, receiver) = sync_channel(50);
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
    future: Pin<&'static mut dyn Future<Output = ()>>,
    join_handle: *const TaskJoinHandle,
}

/// SAFETY: join_handle is only accessed via a mutex
unsafe impl Send for Task {}

impl Task {
    fn new(future: Pin<&mut dyn Future<Output = ()>>) -> Self {
        // SAFETY: run_*() functions always join futures before returning
        let future =
            unsafe { std::mem::transmute::<_, Pin<&'static mut dyn Future<Output = ()>>>(future) };

        Self {
            future,
            join_handle: ptr::null(),
        }
    }

    fn run(mut self: Pin<&mut Self>, join_handle: &Pin<&TaskJoinHandle>) {
        self.join_handle = &**join_handle;

        let message = ExecutorMessage::Task(TaskPtr { inner: &mut *self });

        SYNC_CHANNEL
            .sender
            .try_send(message)
            .expect("executor channel full");
    }

    fn poll_future(&mut self) -> bool {
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

                true
            }
            Poll::Pending => false,
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
    main_task: Arc<AtomicPtr<Task>>,
    task_cvar: Arc<Condvar>,
    task_cvar_mutex: Arc<Mutex<bool>>,
}

enum ExecutorMessage {
    Task(TaskPtr),
    Join,
}

impl Executor {
    pub fn new<F>(register_thread: F) -> (Self, Vec<ThreadId>)
    where
        F: Fn() + Send + 'static,
    {
        const NUM_THREADS: usize = 4;

        let main_task = Arc::new(AtomicPtr::new(ptr::null_mut()));

        let task_cvar = Arc::new(Condvar::new());
        let task_cvar_mutex = Arc::new(Mutex::new(false));

        let mut thread_join_handles = Vec::with_capacity(NUM_THREADS);

        let thread_ids = Mutex::new(Vec::new());
        // SAFETY: we only ever reference this while thread_ids is still in scope
        let thread_ids_ref =
            unsafe { std::mem::transmute::<_, &'static Mutex<Vec<ThreadId>>>(&thread_ids) };

        let register_thread = Arc::new(Mutex::new(register_thread));

        for _ in 0..NUM_THREADS {
            let local_register_thread = register_thread.clone();
            let main_task = main_task.clone();
            let task_cvar = task_cvar.clone();
            let task_cvar_mutex = task_cvar_mutex.clone();
            thread_join_handles.push(thread::spawn(move || {
                thread_ids_ref.lock().unwrap().push(thread::current().id());

                local_register_thread.lock().unwrap()();
                drop(local_register_thread);

                loop {
                    match SYNC_CHANNEL.receiver.lock().unwrap().recv().unwrap() {
                        ExecutorMessage::Task(task_ptr) => {
                            // SAFETY: we only ever create a Task reference here
                            let task = unsafe { task_ptr.inner.as_mut().unwrap() };
                            if task.poll_future()
                                && task_ptr.inner == main_task.load(Ordering::Acquire)
                            {
                                let mut task_guard = task_cvar_mutex.lock().unwrap();
                                *task_guard = true;
                                task_cvar.notify_one();
                            }
                        }
                        ExecutorMessage::Join => break,
                    }
                }
            }));
        }

        while thread_ids.lock().unwrap().len() < NUM_THREADS {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        let executor = Self {
            thread_join_handles,
            main_task,
            task_cvar,
            task_cvar_mutex,
        };

        (executor, thread_ids.into_inner().unwrap())
    }

    pub fn execute_blocking(&mut self, future: &mut (dyn Future<Output = ()> + Send)) {
        // guaranteed not to move in the scope of this function
        let future = unsafe { Pin::new_unchecked(future) };

        let mut task = Task::new(future);
        let mut task = unsafe { Pin::new_unchecked(&mut task) };

        let join_handle = TaskJoinHandle::new();
        let join_handle = unsafe { Pin::new_unchecked(&join_handle) };

        self.main_task.store(&mut *task, Ordering::Release);

        task.run(&join_handle);

        let mut task_guard = self
            .task_cvar
            .wait_while(self.task_cvar_mutex.lock().unwrap(), |done| !*done)
            .unwrap();

        *task_guard = false;
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

macro_rules! pin_array_mut {
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

pub async fn run_batch<F: Fn(usize) + Sync, const N: usize>(f: F) {
    let f = &f;
    let f_async = |index: usize| async move { f(index) };

    let mut futures = unsafe {
        let mut futures: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for (i, future) in futures.iter_mut().enumerate() {
            future.write(f_async(i));
        }
        futures.map(|a| a.assume_init())
    };
    pin_array_mut!(futures, N);

    let mut tasks = unsafe {
        let mut tasks: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for (i, future) in futures.into_iter().enumerate() {
            tasks[i].write(Task::new(future));
        }
        tasks.map(|a| a.assume_init())
    };
    pin_array_mut!(tasks, N);

    let mut join_handles = unsafe {
        let mut join_handles: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for i in 0..N {
            join_handles[i].write(TaskJoinHandle::new());
        }
        join_handles.map(|a| a.assume_init())
    };
    pin_array_mut!(join_handles, N);

    for (i, task) in tasks.into_iter().enumerate() {
        task.run(&join_handles[i].as_ref());
    }

    for join_handle in join_handles {
        join_handle.await;
    }
}

pub async fn run_parallel<const N: usize>(futures: [&mut (dyn Future<Output = ()> + Send); N]) {
    let futures = unsafe { futures.map(|a| Pin::new_unchecked(a)) };

    let mut tasks = unsafe {
        let mut tasks: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for (i, future) in futures.into_iter().enumerate() {
            tasks[i].write(Task::new(future));
        }
        tasks.map(|a| a.assume_init())
    };
    pin_array_mut!(tasks, N);

    let mut join_handles = unsafe {
        let mut join_handles: [MaybeUninit<_>; N] = MaybeUninit::uninit().assume_init();
        for i in 0..N {
            join_handles[i].write(TaskJoinHandle::new());
        }
        join_handles.map(|a| a.assume_init())
    };
    pin_array_mut!(join_handles, N);

    for (i, task) in tasks.into_iter().enumerate() {
        task.run(&join_handles[i].as_ref());
    }

    for join_handle in join_handles {
        join_handle.await;
    }
}

struct PtrWrapper<T: Send>(*const T);

unsafe impl<T: Send> Send for PtrWrapper<T> {}

impl<T: Send> Copy for PtrWrapper<T> {}

impl<T: Send> Clone for PtrWrapper<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

struct MutPtrWrapper<T: Send>(*mut T);

unsafe impl<T: Send> Send for MutPtrWrapper<T> {}

impl<T: Send> Copy for MutPtrWrapper<T> {}

impl<T: Send> Clone for MutPtrWrapper<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

pub async fn run_slice<T: Send, F: Fn(&T) + Sync>(slice: &[T], f: F) {
    const CONCURRENCY: usize = 8;
    const STRIDE: usize = 8;

    let f = &f;
    let slice_ptr = PtrWrapper(slice.as_ptr());
    let slice_len = slice.len();

    let f_async = |offset: usize, slice_ptr: PtrWrapper<T>| {
        if offset < slice_len {
            let len = usize::min(STRIDE, slice_len - offset);
            for i in 0..len {
                f(unsafe { &*slice_ptr.0.add(offset + i) });
            }
        }
    };

    let mut offset = 0;
    while offset < slice_len {
        let mut futures = unsafe {
            let mut futures: [MaybeUninit<_>; CONCURRENCY] = MaybeUninit::uninit().assume_init();
            for future in futures.iter_mut() {
                future.write(async move {
                    f_async(offset, slice_ptr);
                });
                offset += STRIDE;
            }
            futures.map(|a| a.assume_init())
        };
        pin_array_mut!(futures, CONCURRENCY);

        let mut tasks = unsafe {
            let mut tasks: [MaybeUninit<_>; CONCURRENCY] = MaybeUninit::uninit().assume_init();
            for (i, future) in futures.into_iter().enumerate() {
                tasks[i].write(Task::new(future));
            }
            tasks.map(|a| a.assume_init())
        };
        pin_array_mut!(tasks, CONCURRENCY);

        let mut join_handles = unsafe {
            let mut join_handles: [MaybeUninit<_>; CONCURRENCY] =
                MaybeUninit::uninit().assume_init();
            for i in 0..CONCURRENCY {
                join_handles[i].write(TaskJoinHandle::new());
            }
            join_handles.map(|a| a.assume_init())
        };
        pin_array_mut!(join_handles, CONCURRENCY);

        for (i, task) in tasks.into_iter().enumerate() {
            task.run(&join_handles[i].as_ref());
        }

        for join_handle in join_handles {
            join_handle.await;
        }
    }
}

pub async fn run_slice_mut<T: Send, F: Fn(&mut T) + Sync>(slice: &mut [T], f: F) {
    const CONCURRENCY: usize = 8;
    const STRIDE: usize = 8;

    let f = &f;
    let slice_ptr = MutPtrWrapper(slice.as_mut_ptr());
    let slice_len = slice.len();

    let f_async = |offset: usize, slice_ptr: MutPtrWrapper<T>| {
        if offset < slice_len {
            let len = usize::min(STRIDE, slice_len - offset);
            for i in 0..len {
                f(unsafe { &mut *slice_ptr.0.add(offset + i) });
            }
        }
    };

    let mut offset = 0;
    while offset < slice_len {
        let mut futures = unsafe {
            let mut futures: [MaybeUninit<_>; CONCURRENCY] = MaybeUninit::uninit().assume_init();
            for future in futures.iter_mut() {
                future.write(async move {
                    f_async(offset, slice_ptr);
                });
                offset += STRIDE;
            }
            futures.map(|a| a.assume_init())
        };
        pin_array_mut!(futures, CONCURRENCY);

        let mut tasks = unsafe {
            let mut tasks: [MaybeUninit<_>; CONCURRENCY] = MaybeUninit::uninit().assume_init();
            for (i, future) in futures.into_iter().enumerate() {
                tasks[i].write(Task::new(future));
            }
            tasks.map(|a| a.assume_init())
        };
        pin_array_mut!(tasks, CONCURRENCY);

        let mut join_handles = unsafe {
            let mut join_handles: [MaybeUninit<_>; CONCURRENCY] =
                MaybeUninit::uninit().assume_init();
            for i in 0..CONCURRENCY {
                join_handles[i].write(TaskJoinHandle::new());
            }
            join_handles.map(|a| a.assume_init())
        };
        pin_array_mut!(join_handles, CONCURRENCY);

        for (i, task) in tasks.into_iter().enumerate() {
            task.run(&join_handles[i].as_ref());
        }

        for join_handle in join_handles {
            join_handle.await;
        }
    }
}
