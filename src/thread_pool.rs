use std::{
    fmt::Debug,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Instant,
};

pub struct ThreadPool<T: 'static, E: Debug + 'static> {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job<T, E>>>,
}

type Job<T, E> = Box<dyn FnOnce() -> Result<T, E> + Send + 'static>;

impl<T: 'static, E: Debug + 'static> ThreadPool<T, E> {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel::<Job<T, E>>();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
    {
        let job = Box::new(f);

        self.sender
            .as_ref()
            .expect("Failed to get job sender")
            .send(job)
            .expect("Failed to send job");
    }
}

impl<T: 'static, E: 'static + Debug> Drop for ThreadPool<T, E> {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().expect("Failed to join worker thread");
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new<T: 'static, E: Debug + 'static>(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<Job<T, E>>>>,
    ) -> Self {
        let thread = thread::spawn(move || loop {
            let message = receiver
                .lock()
                .expect("Failed to acquire lock on job receiver")
                .recv();

            if let Ok(job) = message {
                println!("Worker {id} got a job; executing.");

                let now = Instant::now();
                let res = job();
                let elapsed_time = now.elapsed();

                match res {
                    Ok(_) => println!(
                        "Worker {id} finished job successfully in {}ms.",
                        elapsed_time.as_millis()
                    ),
                    Err(e) => println!("Worker {id} encountered an error executing job: {e:#?}"),
                }
            } else {
                println!("Worker {id} disconnected; shutting down.");
                break;
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}
