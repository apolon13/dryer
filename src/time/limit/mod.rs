use std::time::{Duration, Instant};

pub struct OnceIn {
    interval: Duration,
    last: Option<Instant>,
}

impl OnceIn {

    pub fn new(interval: Duration) -> Self {
        Self { interval, last: None}
    }
    
    pub fn if_allow<F: FnMut() -> Result<(), anyhow::Error>>(&mut self, mut cb: F) -> Result<(), anyhow::Error> {
        let current = Instant::now();
        if self.last.is_none() || (current.duration_since(self.last.unwrap()) >= self.interval) {
            self.last = Option::from(Instant::now());
            cb()?;
        }
        Ok(())
    }
}
