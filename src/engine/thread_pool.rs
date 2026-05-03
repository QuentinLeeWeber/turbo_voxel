use crossbeam::channel::{Receiver, Sender, unbounded};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

type Task<T> = Box<dyn FnOnce() -> T + Send + 'static>;

pub struct ThreadPool<R: Send + 'static> {
    task_tx: Sender<Task<R>>,
    result_rx: Receiver<R>,
    _workers: Vec<thread::JoinHandle<()>>,
    task_count: Arc<AtomicUsize>,
}

impl<R: Send + 'static> ThreadPool<R> {
    pub fn new(num_workers: usize) -> Self {
        let (task_tx, task_rx) = unbounded::<Task<R>>();
        let (result_tx, result_rx) = unbounded::<R>();
        let mut workers = Vec::new();
        let task_count = Arc::new(AtomicUsize::new(0));

        for _ in 0..num_workers {
            let task_rx = task_rx.clone();
            let result_tx = result_tx.clone();
            let task_count_cloned = task_count.clone();
            let handle = thread::spawn(move || {
                while let Ok(task) = task_rx.recv() {
                    // task taken from queue -> decrement when done
                    let res = task();
                    let _ = result_tx.send(res);
                    task_count_cloned.fetch_sub(1, Ordering::SeqCst);
                }
            });
            workers.push(handle);
        }

        ThreadPool {
            task_tx,
            result_rx,
            _workers: workers,
            task_count,
        }
    }

    pub fn add_task<F>(&self, f: F)
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.task_count.fetch_add(1, Ordering::SeqCst);
        let task: Task<R> = Box::new(move || f());
        let _ = self.task_tx.send(task);
    }

    pub fn results(&self) -> Receiver<R> {
        self.result_rx.clone()
    }

    pub fn task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread::sleep, time::Duration};

    #[test]
    fn basic_submit_and_receive() {
        let pool = ThreadPool::<u32>::new(4);
        let rx = pool.results();

        pool.add_task(|| 1u32);
        pool.add_task(|| 2u32);
        pool.add_task(|| 3u32);

        let mut results = vec![];
        for _ in 0..3 {
            results.push(rx.recv().expect("should receive result"));
        }
        results.sort();
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[test]
    fn concurrent_workload() {
        let pool = ThreadPool::<usize>::new(8);
        let rx = pool.results();

        for i in 0..100 {
            pool.add_task(move || i * i);
        }

        let mut sum = 0usize;
        for _ in 0..100 {
            sum += rx.recv().unwrap();
        }

        let expected = (99usize * 100usize * 199usize) / 6;
        assert_eq!(sum, expected);
    }

    #[test]
    fn heterogeneous_timing() {
        let pool = ThreadPool::<u8>::new(3);
        let rx = pool.results();

        pool.add_task(|| {
            std::thread::sleep(Duration::from_millis(50));
            1u8
        });
        pool.add_task(|| 2u8);
        pool.add_task(|| {
            std::thread::sleep(Duration::from_millis(10));
            3u8
        });

        let mut got = vec![];
        for _ in 0..3 {
            got.push(rx.recv().unwrap());
        }
        got.sort();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn task_count_only() {
        let pool = ThreadPool::<()>::new(4);
        let rx = pool.results();

        assert_eq!(pool.task_count(), 0);

        for _ in 0..5 {
            pool.add_task(move || sleep(Duration::from_millis(5)));
        }
        assert_eq!(pool.task_count(), 5);

        let mut got = Vec::new();
        for _ in 0..5 {
            got.push(rx.recv().expect("should receive result"));
        }

        sleep(Duration::from_millis(10));
        assert_eq!(pool.task_count(), 0);
    }
}
