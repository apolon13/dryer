use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Relaxed, Release};
use crate::mqtt::MqttMessage;

pub mod sensor;
pub mod fan;
pub mod heater;

#[derive(Debug)]
pub struct State {
    val: AtomicBool,
}

impl State {
    pub fn new() -> Self {
        Self { val: AtomicBool::new(false) }
    }

    pub fn activate(&self) {
        self.val.store(true, Relaxed);
    }

    pub fn inactivate(&self) {
        self.val.store(false, Relaxed);
    }

    pub fn active(&self) -> bool {
        self.val.load(Relaxed)
    }
}

pub struct StateMessage {
    active: bool,
}

impl StateMessage {
    pub fn new(state: bool) -> Self {
        Self { active: state }
    }
}

impl MqttMessage for StateMessage {
    fn to_bytes(&self) -> &[u8] {
        if self.active {
            return "true".as_bytes();
        }
        "false".as_bytes()
    }

    fn topic(&self) -> &str {
        "/state/status"
    }
}