use std::cmp;
use std::cmp::PartialEq;
use anyhow::Error;
use esp_idf_hal::ledc::LedcDriver;
use crate::dryer::FanSpeedRegulator;

pub struct Fan<'a> {
    pwm: LedcDriver<'a>,
    duty_step: u32,
}

#[derive(PartialEq)]
enum Direction {
    Up,
    Down
}

impl<'a> Fan<'a> {
    pub(crate) fn new(pwm: LedcDriver<'a>) -> Self {
        Fan { pwm, duty_step: 200 }
    }

    fn next_step(&self, direction: Direction) -> u32 {
        let max = self.pwm.get_max_duty();
        let current = self.pwm.get_duty();
        match direction {
            Direction::Up => {
                cmp::min(max, current.saturating_add(self.duty_step))
            }
            Direction::Down => {
                cmp::max(0, current.saturating_sub(self.duty_step))
            }
        }
    }
}

impl<'a> FanSpeedRegulator for Fan<'a> {
    fn more_speed(&mut self) -> Result<(), Error> {
        self.pwm.set_duty(self.next_step(Direction::Up))?;
        Ok(())
    }

    fn less_speed(&mut self) -> Result<(), Error> {
        self.pwm.set_duty(self.next_step(Direction::Down))?;
        Ok(())
    }
}
