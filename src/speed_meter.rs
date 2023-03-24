use std::{collections::LinkedList, sync::Mutex, time::Instant};

pub struct SpeedMeter {
    sample_count: usize,

    lock: Mutex<bool>,

    last_sample_time: std::time::Instant,
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
        let _lock = self.lock.lock().unwrap();
        self.last_sample_time = Instant::now();
        self.samples.clear();
    }

    pub fn get_speed(&self) -> f64 {
        let _lock = self.lock.lock().unwrap();

        let mut sum = 0.0;
        for sample in self.samples.iter() {
            sum += sample;
        }

        sum / self.samples.len() as f64
    }

    pub fn log(&mut self, size: usize) {
        let _lock = self.lock.lock().unwrap();

        let now = Instant::now();
        let elapsed = now - self.last_sample_time;
        self.last_sample_time = now;

        let elapsed = elapsed.as_millis() as f64;
        let speed = size as f64 / elapsed * 1000f64;

        self.samples.push_back(speed);

        if self.samples.len() > self.sample_count {
            self.samples.pop_front();
        }
    }
}
