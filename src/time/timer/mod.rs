use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;
use chrono::NaiveDateTime;
use crate::schedule::Timer;

pub struct SyncTimer {
    done_ch: Receiver<bool>,
    start: NaiveDateTime
}

impl SyncTimer {
    pub fn new(done_ch: Receiver<bool>, start: NaiveDateTime) -> Self {
        Self{start, done_ch}
    }
}

impl Timer for SyncTimer {
    fn next_sec<F: FnMut(NaiveDateTime)>(&self, mut cb: F) {
        let mut current_time = self.start;
        loop {
            match self.done_ch.try_recv() {
                Ok(true) => { return }
                _ => {}
            };
            sleep(Duration::from_secs(1));
            current_time = current_time + Duration::from_secs(1);
            cb(current_time);
        }
    }
}