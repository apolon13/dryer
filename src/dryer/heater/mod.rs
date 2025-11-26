use crate::dryer::State;
use crate::time::timer::SyncTimer;
use anyhow::{anyhow, Error};
use crossbeam_channel::Sender;
use embedded_hal::digital::OutputPin;

pub trait TempSensor {
    fn read_celsius(&mut self) -> anyhow::Result<u16, Error>;
}

#[derive(PartialEq)]
pub enum FanSpeed {
    Middle,
    Max,
    Low,
    Off,
}

pub trait FanSpeedRegulator {
    fn speed(&mut self, speed: FanSpeed) -> anyhow::Result<(), Error>;
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
        self.fan.speed(FanSpeed::Low)?;
        Ok(())
    }

    fn dry(&mut self) -> anyhow::Result<(), Error> {
        self.power_on()?;
        self.fan.speed(FanSpeed::Middle)?;
        Ok(())
    }

    fn cooling(&mut self) -> anyhow::Result<(), Error> {
        self.power_off()?;
        self.fan.speed(FanSpeed::Max)?;
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
        self.fan.speed(FanSpeed::Off)
    }

    pub fn start(&mut self, timer: SyncTimer, state: Sender<State>) -> Result<(), Error> {
        let mut failed_requests = 0;
        let mut target_reached = false;
        timer.next_sec(|| {
            if failed_requests > 30 {
                Err(anyhow!("too many failed temperature requests"))?
            }
            match self.sensor.read_celsius() {
                Ok(value) => {
                    let mut action = "";
                    failed_requests = 0;
                    let target = self.target_temperature;
                    let min = target - 5;
                    let max = self.target_temperature + 10;
                    if target_reached && value.lt(&min) {
                        target_reached = false;
                    }
                    if value.lt(&target) && !target_reached {
                        action = "heat";
                        self.heat()?;
                    }
                    if (target..max).contains(&value) || target_reached {
                        target_reached = true;
                        action = "dry";
                        self.dry()?;
                    }
                    if value.gt(&max) {
                        target_reached = false;
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
