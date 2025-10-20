use anyhow::{anyhow, Error};
use embedded_hal::digital::OutputPin;
use crate::time::timer::SyncTimer;

pub enum Err {
    ContextCanceled,
}

pub trait TempSensor {
    fn read_celsius(&mut self) -> anyhow::Result<u16, Error>;
}

#[derive(PartialEq)]
pub enum FanMode {
    Max,
    Middle,
    Off
}

pub trait FanSpeedRegulator {
    fn speed(&mut self, mode: FanMode) -> anyhow::Result<(), Error>;
}

pub struct Heater<P: OutputPin, S: TempSensor, F: FanSpeedRegulator> {
    power: P,
    sensor: S,
    fan: F,
    target_temperature: u8
}

impl<P: OutputPin, S: TempSensor, F: FanSpeedRegulator> Heater<P, S, F> {
    pub fn new(power: P, target_temperature: u8, sensor: S, fan: F) -> Self {
        Heater { power, target_temperature, sensor, fan }
    }

    fn heat(&mut self) -> anyhow::Result<(), Error> {
        self.power.set_high().map_err(|e| {anyhow!("power on: {:?}", e)})?;
        self.fan.speed(FanMode::Off)?;
        Ok(())
    }

    fn dry(&mut self) -> anyhow::Result<(), Error> {
        self.fan.speed(FanMode::Middle)?;
        Ok(())
    }

    fn cooling(&mut self) -> anyhow::Result<(), Error> {
        self.power_off()?;
        self.fan.speed(FanMode::Off)?;
        Ok(())
    }

    fn power_off(&mut self) -> anyhow::Result<(), Error> {
        self.power.set_low().map_err(|e| {anyhow!("power off: {:?}", e)})?;
        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<(), Error> {
        self.power_off()
    }

    pub fn start(
        &mut self,
        timer: SyncTimer
    ) -> Result<(), Error> {
        let mut failed_requests = 0;
        timer.next_sec(|| {
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
            Ok(())
        })
    }
}