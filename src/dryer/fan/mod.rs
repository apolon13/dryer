use crate::dryer::heater::{FanMode, FanSpeedRegulator};
use anyhow::Error;
use esp_idf_hal::ledc::LedcDriver;

pub struct Fan<'a> {
    pwm: LedcDriver<'a>,
}

impl<'a> Fan<'a> {
    pub fn new(pwm: LedcDriver<'a>) -> Self {
        Fan { pwm }
    }

    fn next_step_for_dryer(&self, direction: FanMode) -> u32 {
        let max = self.pwm.get_max_duty();
        match direction {
            FanMode::Middle => (max as f64 * (1.0 - 30.0 / 100.0)) as u32,
            FanMode::Max => max,
            FanMode::Off => 0,
        }
    }
}

impl<'a> FanSpeedRegulator for Fan<'a> {
    fn speed(&mut self, mode: FanMode) -> Result<(), Error> {
        self.pwm.set_duty(self.next_step_for_dryer(mode))?;
        Ok(())
    }
}
