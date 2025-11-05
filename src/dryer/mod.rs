use serde::{Serialize};
use crate::mqtt::MqttMessage;

pub mod sensor;
pub mod fan;
pub mod heater;

#[derive(Debug, Serialize)]
pub struct State {
    active: bool,
    temp: u16,
    action: String,
}

impl State {
    pub fn new(active: bool, temp: u16, action: String) -> Self {
        Self { active, temp, action }
    }

    pub fn active() -> Self {
        Self { active: true, temp: 0, action: String::new() }
    }

    pub fn inactive() -> Self {
        Self { active: false, temp: 0, action: String::new() }
    }
}

impl MqttMessage for State {
    fn to_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self)?)
    }

    fn topic(&self) -> &str {
        "/state"
    }
}