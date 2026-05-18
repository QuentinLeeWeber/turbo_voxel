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
    pub fn new(num_workers: usize, name: Option<String>) -> Self {
        let (task_tx, task_rx) = unbounded::<Task<R>>();
        let (result_tx, result_rx) = unbounded::<R>();
        let mut workers = Vec::new();
        let task_count = Arc::new(AtomicUsize::new(0));

        for i in 0..num_workers {
            let task_rx = task_rx.clone();
            let result_tx = result_tx.clone();
            let task_count_cloned = task_count.clone();
            let handle = thread::Builder::new()
                .name(format!(
                    "worker {}, of pool {}",
                    i,
                    name.as_deref().unwrap_or("unnamed")
                ))
                .spawn(move || {
                    while let Ok(task) = task_rx.recv() {
                        let res = task();
                        let _ = result_tx.send(res);
                        task_count_cloned.fetch_sub(1, Ordering::SeqCst);
                    }
                })
                .unwrap();
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

    pub fn results(&self) -> Vec<R> {
        self.result_rx.try_iter().collect()
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
        let pool = ThreadPool::<u32>::new(4, None);

        pool.add_task(|| 1u32);
        pool.add_task(|| 2u32);
        pool.add_task(|| 3u32);

        sleep(Duration::from_millis(2));

        let mut results = pool.results();
        results.sort();
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[test]
    fn test_recv_non_blocking_1() {
        let pool = ThreadPool::<()>::new(4, None);
        let results = pool.results();

        assert!(results.is_empty());
    }

    #[test]
    fn test_recv_non_blocking_2() {
        let pool = ThreadPool::<()>::new(4, None);

        let results = pool.results();
        assert!(results.is_empty());

        pool.add_task(|| {
            sleep(Duration::from_millis(5));
        });
        sleep(Duration::from_millis(10));

        let results = pool.results();
        assert!(!results.is_empty());
    }

    #[test]
    fn concurrent_workload() {
        let pool = ThreadPool::<usize>::new(8, None);

        for i in 0..100 {
            pool.add_task(move || i * i);
        }

        sleep(Duration::from_millis(5));

        let sum: usize = pool.results().into_iter().sum();

        let expected = (99usize * 100usize * 199usize) / 6;
        assert_eq!(sum, expected);
    }

    #[test]
    fn heterogeneous_timing() {
        let pool = ThreadPool::<u8>::new(3, None);

        pool.add_task(|| {
            sleep(Duration::from_millis(5));
            1u8
        });
        pool.add_task(|| 2u8);
        pool.add_task(|| {
            sleep(Duration::from_millis(1));
            3u8
        });

        sleep(Duration::from_millis(10));

        let mut got = pool.results();

        got.sort();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn task_count_only() {
        let pool = ThreadPool::<()>::new(4, None);

        assert_eq!(pool.task_count(), 0);

        for _ in 0..5 {
            pool.add_task(move || sleep(Duration::from_millis(2)));
        }
        assert_eq!(pool.task_count(), 5);

        sleep(Duration::from_millis(10));

        let _got = pool.results();

        sleep(Duration::from_millis(4));
        assert_eq!(pool.task_count(), 0);
    }
}
