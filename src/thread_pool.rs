use std::{
    sync::{
        mpsc::{self, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::Instant,
};

pub struct ThreadPool<T: 'static, E: 'static, R: Send> {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job<T, E>>>,
    err_receiver: mpsc::Receiver<R>,
}

type Job<T, E> = Box<dyn FnOnce() -> Result<T, E> + Send + 'static>;

impl<T: 'static, E: 'static, R: Send + 'static> ThreadPool<T, E, R> {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(size: usize, err_handler: fn(E) -> R) -> Self {
        assert!(size > 0);

        let (job_sender, job_receiver) = mpsc::channel::<Job<T, E>>();
        let (err_sender, err_receiver) = mpsc::channel::<R>();

        let job_receiver = Arc::new(Mutex::new(job_receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(
                id,
                Arc::clone(&job_receiver),
                err_sender.clone(),
                err_handler,
            ));
        }

        Self {
            workers,
            sender: Some(job_sender),
            err_receiver,
        }
    }

    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::missing_errors_doc)]
    pub fn execute<F>(&self, f: F) -> Result<R, TryRecvError>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
    {
        let job = Box::new(f);

        self.sender
            .as_ref()
            .expect("Failed to get job sender")
            .send(job)
            .expect("Failed to send job");

        self.err_receiver.try_recv().map_err(|e| match e {
            TryRecvError::Empty => e,
            TryRecvError::Disconnected => panic!("Worker disconnected"),
        })
    }
}

impl<T: 'static, E: 'static, R: Send> Drop for ThreadPool<T, E, R> {
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
    fn new<T: 'static, E: 'static, R: Send + 'static>(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<Job<T, E>>>>,
        err_sender: mpsc::Sender<R>,
        err_handler: fn(E) -> R,
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
                    Err(e) => {
                        println!("Worker {id} encountered an error; handling.");
                        err_sender
                            .send(err_handler(e))
                            .unwrap_or_else(|_| panic!("Failed to handle error in worker {id}"));
                    }
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
