use crate::dryer::State;
use crate::time::timer::SyncTimer;
use anyhow::{anyhow, Error};
use crossbeam_channel::Sender;
use embedded_hal::digital::OutputPin;

pub trait TempSensor {
    fn read_celsius(&mut self) -> anyhow::Result<u16, Error>;
}

#[derive(PartialEq)]
pub enum FanMode {
    Max,
    Middle,
    Off,
}

pub trait FanSpeedRegulator {
    fn speed(&mut self, mode: FanMode) -> anyhow::Result<(), Error>;
}

pub struct Heater<P: OutputPin, S: TempSensor, F: FanSpeedRegulator> {
    power: P,
    sensor: S,
    fan: F,
    target_temperature: u16,
}

impl<P: OutputPin, S: TempSensor, F: FanSpeedRegulator> Heater<P, S, F> {
    pub fn new(power: P, target_temperature: u16, sensor: S, fan: F) -> Self {
        Heater {
            power,
            target_temperature,
            sensor,
            fan,
        }
    }

    fn heat(&mut self) -> anyhow::Result<(), Error> {
        self.power_on()?;
        self.fan.speed(FanMode::Middle)?;
        Ok(())
    }

    fn dry(&mut self) -> anyhow::Result<(), Error> {
        self.power_on()?;
        self.fan.speed(FanMode::Max)?;
        Ok(())
    }

    fn cooling(&mut self) -> anyhow::Result<(), Error> {
        self.power_off()?;
        self.fan.speed(FanMode::Max)?;
        Ok(())
    }

    fn power_on(&mut self) -> anyhow::Result<(), Error> {
        self.power
            .set_high()
            .map_err(|e| anyhow!("power on: {:?}", e))
    }

    fn power_off(&mut self) -> anyhow::Result<(), Error> {
        self.power
            .set_low()
            .map_err(|e| anyhow!("power off: {:?}", e))
    }

    pub fn stop(&mut self) -> anyhow::Result<(), Error> {
        self.power_off()?;
        self.fan.speed(FanMode::Off)
    }

    pub fn start(&mut self, timer: SyncTimer, state: Sender<State>) -> Result<(), Error> {
        let mut failed_requests = 0;
        timer.next_sec(|| {
            if failed_requests > 30 {
                Err(anyhow!("too many failed temperature requests"))?
            }
            match self.sensor.read_celsius() {
                Ok(value) => {
                    let mut action = "";
                    failed_requests = 0;
                    let min = self.target_temperature;
                    let max = self.target_temperature + 10;
                    if value.lt(&min) {
                        action = "heat";
                        self.heat()?;
                    }
                    if (min..max).contains(&value) {
                        action = "dry";
                        self.dry()?;
                    }
                    if value.gt(&max) {
                        action = "cooling";
                        self.cooling()?;
                    }
                    state.try_send(State::new(true, value, action.to_string()))?;
                }
                _ => {
                    failed_requests += 1;
                }
            }
            Ok(())
        })
    }
}
