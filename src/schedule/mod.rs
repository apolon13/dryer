use chrono::{NaiveDateTime, Timelike};
use std::collections::HashMap;
use uuid::{Uuid};

pub struct PeriodicJob {
    uuid: Uuid,
    run_at_hour: u32,
    cb: Box<dyn Fn()>,
}

impl PeriodicJob {
    pub fn new(run_at_hour: u32, cb: Box<dyn Fn()>) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            run_at_hour,
            cb,
        }
    }

    pub fn time_is_come(&self, current_hour: u32) -> bool {
        self.run_at_hour == current_hour
    }

    pub fn run(&self) {
        (self.cb)();
    }
}

pub trait Timer {
    fn next_sec<F: FnMut(NaiveDateTime)>(&self, cb: F);
}

pub struct Scheduler {}

impl Scheduler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run<T: Timer>(&self, timer: T, jobs: Vec<PeriodicJob>) {
        let mut happened_map: HashMap<Uuid, bool> = HashMap::new();
        timer.next_sec(|dt| {
            for job in jobs.iter() {
                match happened_map.get(&job.uuid) {
                    Some(happened) => {
                        if job.time_is_come(dt.hour()) {
                            if !*happened {
                                job.run();
                                happened_map.insert(job.uuid, true);
                            }
                        } else {
                            happened_map.insert(job.uuid, false);
                        }
                    }
                    None => {
                        if job.time_is_come(dt.hour()) {
                            job.run();
                            happened_map.insert(job.uuid, true);
                        }
                    }
                }
            }
        });
    }
}
