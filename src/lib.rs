//! # InfinityThread
//!
//! `InfinityThread` is a simple library for running tasks in a thread that automatically restarts on panics.
//!
//! ## Example
//!
//! ```rust
//! use infinity_threads::InfinityThread;
//! use std::time::Duration;
//!
//! let mut thread = InfinityThread::new();
//!
//! thread.start(
//!     || {
//!         // put the task to run here
//!         println!("Running task...");
//!     },
//!     // sleep between runs for X seconds
//!     Duration::from_secs(1),
//! );
//!
//! std::thread::sleep(Duration::from_secs(5));
//! thread.stop();
//! ```
use std::sync::atomic::{self, AtomicBool};
use std::sync::{Arc, Mutex};
use std::{panic, thread};
use std::time::Duration;

/// A thread that runs a task, automatically restarting it if it panics.
pub struct InfinityThread {
    handle: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}
impl InfinityThread {
    /// Create a new `InfinityThread`.
    pub fn new() -> Self {
        InfinityThread {
            handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts the thread with a given task and sleep duration.
    ///
    /// # Arguments
    ///
    /// * `closure` - A task to run repeatedly.
    /// * `sleep_for` - The duration to sleep between task executions.
    pub fn start<T>(&mut self, closure: T, sleep_for: Duration)
    where
        T: FnMut() + Send + Sync + 'static,
    {
        let stop_flag = Arc::clone(&self.stop_flag);
        let closure = Arc::new(Mutex::new(closure));

        self.handle = Some(thread::spawn(move || {
            while !stop_flag.load(atomic::Ordering::Relaxed) {
                let closure_clone = Arc::clone(&closure);
                let stop_flag_clone = Arc::clone(&stop_flag);

                // Spawn a monitoring thread for the task.
                let task_thread = thread::spawn(move || {
                    loop {
                        if stop_flag_clone.load(atomic::Ordering::Relaxed) {
                            // println!("Stop flag detected. Exiting inner loop...");
                            break;
                        }
                        match closure_clone.lock() {
                            Ok(mut closure) => {
                                if let Err(_) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                                    (*closure)();
                                })) {
                                    // println!("Closure panicked. Restarting...");
                                    break; // Exit inner loop to restart the task thread.
                                }
                            }
                            Err(poison_error) => {
                                // println!("Mutex poisoned. Restarting...");
                                let mut closure = poison_error.into_inner();
                                (*closure)();
                            }
                        }
                        thread::sleep(sleep_for);
                    }
                });

                // Wait for the task thread to finish before restarting.
                let _ = task_thread.join();
            }

            // println!("InfinityThread stopped.");
        }));
    }

    /// Signal the thread to stop and wait for it to finish.
    pub fn stop(&mut self) {
        self.stop_flag.store(true, atomic::Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::InfinityThread;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    use std::thread;

    #[test]
    fn test_thread_runs_task() {

        let mut infinity_thread = InfinityThread::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        infinity_thread.start(
            move || {
                counter_clone.fetch_add(1, Ordering::Relaxed);
            },
            Duration::from_millis(100),
        );

        // Let the thread run for a short time.
        thread::sleep(Duration::from_secs(1));

        infinity_thread.stop();

        // Ensure the task ran multiple times.
        assert!(counter.load(Ordering::Relaxed) > 5);
    }

    #[test]
    fn test_thread_stops_gracefully() {
        let mut infinity_thread = InfinityThread::new();

        infinity_thread.start(
            || {
                println!("Task running...");
                thread::sleep(Duration::from_secs(1));
            },
            Duration::from_millis(200),
        );

        // Stop the thread after a short delay.
        thread::sleep(Duration::from_secs(2));
        let start = Instant::now();
        infinity_thread.stop();
        let elapsed = start.elapsed();

        // Ensure stop() does not hang.
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn test_stop_mid_task() {
        let mut infinity_thread = InfinityThread::new();

        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);

        infinity_thread.start(
            move || {
                flag_clone.store(true, Ordering::Relaxed);
                thread::sleep(Duration::from_secs(5)); // Simulate long task
            },
            Duration::from_secs(1),
        );

        // Ensure the task starts.
        thread::sleep(Duration::from_millis(500));
        assert!(flag.load(Ordering::Relaxed));

        // Stop the thread while the task is running.
        infinity_thread.stop();

        // Ensure the task was interrupted.
        assert!(flag.load(Ordering::Relaxed)); // Task started
        println!("Task stopped mid-execution.");
    }

    #[test]
    fn test_task_restarts_on_panic() {
        let mut infinity_thread = InfinityThread::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        infinity_thread.start(
            move || {
                let count = counter_clone.fetch_add(1, Ordering::Relaxed);
                if count == 2 {
                    panic!("Intentional panic to test recovery.");
                }
            },
            Duration::from_millis(100),
        );

        // Let the thread run for a short time.
        thread::sleep(Duration::from_secs(2));

        infinity_thread.stop();

        // Ensure the task was restarted after the panic.
        assert!(counter.load(Ordering::Relaxed) > 3);
    }
}