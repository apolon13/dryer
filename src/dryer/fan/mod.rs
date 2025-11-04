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
}

impl<'a> FanSpeedRegulator for Fan<'a> {
    fn speed(&mut self, mode: FanMode) -> Result<(), Error> {
        let max = self.pwm.get_max_duty();
        self.pwm.set_duty(match mode {
            FanMode::Middle => (max as f64 * (1.0 - 30.0 / 100.0)) as u32,
            FanMode::Max => max,
            FanMode::Off => 0,
        })?;
        Ok(())
    }
}
