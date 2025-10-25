use serde::{Serialize};
use crate::mqtt::MqttMessage;

pub mod sensor;
pub mod fan;
pub mod heater;

#[derive(Debug, Serialize, Copy, Clone)]
pub struct State {
    active: bool,
    temp: u16
}

impl State {
    pub fn new(active: bool, temp: u16) -> Self {
        Self { active, temp }
    }

    pub fn active() -> Self {
        Self { active: true, temp: 0 }
    }

    pub fn inactive() -> Self {
        Self { active: false, temp: 0 }
    }
}

#[derive(Debug)]
pub struct StateMessage {
    state: State,
}

impl StateMessage {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}

impl MqttMessage for StateMessage {
    fn to_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.state)?)
    }

    fn topic(&self) -> &str {
        "/state"
    }
}