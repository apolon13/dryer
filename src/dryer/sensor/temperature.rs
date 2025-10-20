use anyhow::{anyhow, Error};
use esp_idf_svc::hal::delay::Ets;
use onewire::{DeviceSearch, OneWire, OpenDrainOutput, DS18B20};
use crate::dryer::heater::TempSensor;

pub struct DS18B20Sensor<ODO: OpenDrainOutput> {
    device: DS18B20,
    wire: OneWire<ODO>,
}

impl<ODO: OpenDrainOutput> DS18B20Sensor<ODO> {
    pub fn new(mut wire: OneWire<ODO>, device_search_attempts: i32) -> Result<Self, Error> {
        let mut delay = Ets;
        let mut search = DeviceSearch::new();
        wire.reset(&mut delay)
            .map_err(|e| anyhow!("wire.reset: {:?}", e))?;
        for _ in 0..device_search_attempts {
            let device = wire
                .search_next(&mut search, &mut delay)
                .map_err(|e| anyhow!("wire.search_next: {:?}", e))?;
            if device.is_some() {
                return Ok(DS18B20Sensor {
                    device: DS18B20::new(device.unwrap()).map_err(|e| anyhow!("device.new: {:?}", e))?,
                    wire
                });
            }
        }
        Err(anyhow!("temperature device not found"))
    }
}

impl<ODO: OpenDrainOutput> TempSensor for DS18B20Sensor<ODO> {
    fn read_celsius(&mut self) -> Result<u16, Error> {
        let mut delay = Ets;
        let resolution = self
            .device
            .measure_temperature(&mut self.wire, &mut delay)
            .map_err(|e| anyhow!("device.measure_temperature: {:?}", e))?;
        Ets::delay_ms(resolution.time_ms() as u32);
        let temperature = self
            .device
            .read_temperature(&mut self.wire, &mut delay)
            .map_err(|e| anyhow!("device.read_temperature: {:?}", e))?;
        Ok(temperature / 16.0 as u16)
    }
}
