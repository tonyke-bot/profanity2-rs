use std::{collections::LinkedList, sync::Mutex, time::Instant};

pub struct SpeedMeter {
    sample_count: usize,

    lock: Mutex<bool>,

    last_sample_time: Instant,
    samples: LinkedList<f64>,
}

impl SpeedMeter {
    pub fn new(sample_count: usize) -> Self {
        SpeedMeter {
            sample_count,
            lock: Mutex::new(false),
            samples: LinkedList::new(),
            last_sample_time: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        let lock = self.lock.lock().unwrap();

        self.last_sample_time = Instant::now();
        self.samples.clear();

        drop(lock);
    }

    pub fn get_speed(&self) -> f64 {
        let lock = self.lock.lock().unwrap();

        let sample_count = self.samples.len();
        if sample_count == 0 {
            return 0f64;
        }

        let sum = self.samples.iter().sum::<f64>();

        drop(lock);

        sum / sample_count as f64
    }

    pub fn log(&mut self, size: usize) {
        let lock = self.lock.lock().unwrap();

        let now = Instant::now();
        let elapsed = (now - self.last_sample_time).as_millis() as f64;
        self.last_sample_time = now;

        if elapsed == 0f64 {
            return;
        }

        let speed = size as f64 / elapsed * 1000f64;

        self.samples.push_back(speed);

        if self.samples.len() > self.sample_count {
            self.samples.pop_front();
        }

        drop(lock);
    }
}
