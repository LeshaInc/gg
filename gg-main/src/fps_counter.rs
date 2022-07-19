use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug)]
pub struct FpsCounter {
    samples: VecDeque<f32>,
}

impl FpsCounter {
    pub fn new(capacity: usize) -> FpsCounter {
        FpsCounter {
            samples: VecDeque::with_capacity(capacity),
        }
    }

    pub fn add_sample(&mut self, time: Duration) {
        let time = time.as_secs_f32();

        if self.samples.len() == self.samples.capacity() {
            self.samples.pop_front();
        }

        self.samples.push_back(time)
    }

    pub fn spf(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }

        self.samples.iter().sum::<f32>() / (self.samples.len() as f32)
    }

    pub fn fps(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        1.0 / self.spf()
    }
}
