use std::thread;
use std::time::Duration;
use anyhow::{anyhow, Error, Result};
use embedded_hal::digital::OutputPin;

pub mod sensor;
pub mod fan;

pub trait TemperatureReader {
    fn read_celsius(&mut self) -> Result<u16, Error>;
}

pub trait FanSpeedRegulator {
    fn more_speed(&mut self) -> Result<(), Error>;
    fn less_speed(&mut self) -> Result<(), Error>;
}

pub struct Dryer<P: OutputPin, S: TemperatureReader, F: FanSpeedRegulator> {
    power: P,
    sensor: S,
    fan_reg: F,
    target_temperature: u8
}

impl<P: OutputPin, S: TemperatureReader, F: FanSpeedRegulator> Dryer<P, S, F> {
    pub fn new(power: P, target_temperature: u8, sensor: S, fan_reg: F) -> Self {
        Dryer { power, target_temperature, sensor, fan_reg }
    }

    fn heat(&mut self) -> Result<(), Error> {
        self.power.set_high().map_err(|e| {anyhow!("power on: {:?}", e)})?;
        self.fan_reg.less_speed()?;
        Ok(())
    }

    fn dry(&mut self) -> Result<(), Error> {
        self.fan_reg.more_speed()?;
        Ok(())
    }

    fn cooling(&mut self) -> Result<(), Error> {
        self.power.set_low().map_err(|e| {anyhow!("power off: {:?}", e)})?;
        self.fan_reg.more_speed()?;
        Ok(())
    }

    pub fn start(
        &mut self,
    ) -> Result<(), Error> {
        let mut failed_requests = 0;
        loop {
            if failed_requests > 3 {
                Err(anyhow!("too many failed temperature requests"))?
            }
            match self.sensor.read_celsius() {
                Ok(value) => {
                    failed_requests = 0;
                    let target = self.target_temperature as u16;
                    let target_with_gap = target + 10;
                    if value < target {
                        self.heat()?;
                    }
                    if value >= target && value <= target_with_gap {
                        self.dry()?;
                    }
                    if value > target_with_gap {
                        self.cooling()?;
                    }
                }
                _=> {
                    failed_requests += 1;
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    }
}