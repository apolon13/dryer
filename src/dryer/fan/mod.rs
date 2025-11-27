use crate::dryer::heater::{FanSpeed, FanSpeedRegulator};
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
    fn speed(&mut self, speed: FanSpeed) -> Result<(), Error> {
        let max = self.pwm.get_max_duty();
        self.pwm.set_duty(match speed {
            FanSpeed::Low => {
                (max as f64 * 30.0 / 100.0) as u32
            },
            FanSpeed::Middle => {
                (max as f64 * 50.0 / 100.0) as u32
            },
            FanSpeed::Max => max,
            FanSpeed::Off => 0,
        })?;
        Ok(())
    }
}
