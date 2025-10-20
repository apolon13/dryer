use std::ops::{Add, Sub};
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub struct SyncTimer {
    done_ch: Receiver<bool>,
    passed: Duration
}

impl SyncTimer {
    pub fn new(done_ch: Receiver<bool>, secs: Duration) -> Self {
        Self { done_ch, passed: secs }
    }

    pub fn next_sec<F: FnMut() -> Result<(), anyhow::Error>>(
        &self,
        mut cb: F,
    ) -> Result<(), anyhow::Error> {
        let mut passed = self.passed;
        while self.done_ch.try_recv().is_err() {
            if passed.is_zero() {
                break
            }
            let start = Instant::now();
            cb()?;
            let duration = start.elapsed();
            let sec = Duration::from_secs(1);
            sleep(sec.sub(duration));
            passed = passed.saturating_sub(sec)
        }
        Ok(())
    }
}
