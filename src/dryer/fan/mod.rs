use std::cmp;
use anyhow::Error;
use esp_idf_hal::ledc::LedcDriver;
use crate::dryer::{FanMode, FanSpeedRegulator};

pub struct Fan<'a> {
    pwm: LedcDriver<'a>,
    duty_step: u32,
}

impl<'a> Fan<'a> {
    pub(crate) fn new(pwm: LedcDriver<'a>) -> Self {
        Fan { pwm, duty_step: 200 }
    }

    fn next_step_for_dryer(&self, direction: FanMode) -> u32 {
        let max = self.pwm.get_max_duty();
        let current = self.pwm.get_duty();
        match direction {
            FanMode::Middle => {
                (max as f64 * (1.0 - 30.0 / 100.0)) as u32
            },
            FanMode::Max => {
                cmp::min(max, current.saturating_add(self.duty_step))
            }
            FanMode::Min => {
                cmp::max(0, current.saturating_sub(self.duty_step))
            }
        }
    }
}

impl<'a> FanSpeedRegulator for Fan<'a> {
    fn speed(&mut self, mode: FanMode) -> Result<(), Error> {
        self.pwm.set_duty(self.next_step_for_dryer(mode))?;
        Ok(())
    }
}
