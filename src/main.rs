#[macro_use]
mod schedule;
mod time;
mod wifi;
mod dryer;

use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::wifi::EspWifi;
use log::error;
use onewire::OneWire;
use dryer::sensor::temperature::DS18B20Sensor;
use crate::wifi::{Connection, Credentials};
use dotenv_codegen::dotenv;
use embedded_svc::wifi::AuthMethod;
use esp_idf_hal::delay::Ets;
use dryer::Dryer;
use esp_idf_hal::ledc::{LedcChannel, LedcDriver, LedcTimerDriver, Resolution};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::Resolution::Bits10;
use esp_idf_hal::units::Hertz;
use crate::dryer::fan::Fan;

fn main() -> Result<()> {
    match start() {
        Ok(()) => Ok(()),
        Err(e) => {
            error!("error: {}", e);
            Err(anyhow::anyhow!(e))
        }
    }
}
fn start() -> Result<()> {
    let peripherals = Peripherals::take()?;

    //Init fan
    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default()
            .frequency(Hertz::from(20_000u32))
            .resolution(Bits10),
    )?;
    let pwm = LedcDriver::new(
        peripherals.ledc.channel0,
        timer_driver,
        peripherals.pins.gpio4,
    )?;
    let fan = Fan::new(pwm);

    /*
    //Init WI-FI connection
    let sys_loop = EspSystemEventLoop::take()?;
    let connection = Connection::new(
        Credentials::new(
            dotenv!("SSID").to_string(),
            dotenv!("PASS").to_string(),
        ),
    );
    let mut wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), None)?;
    connection.open(&mut wifi, sys_loop, AuthMethod::WPA2Personal)?;
    */
    
    //Init temperature sensor
    let mut pin_driver = PinDriver::output(peripherals.pins.gpio6)?.into_input_output()?;
    let wire = OneWire::new(&mut pin_driver, false);
    let temp_sensor = DS18B20Sensor::new(wire, 100)?;

    //Init dryer
    let power = PinDriver::output(peripherals.pins.gpio2)?.into_output()?;
    let mut dryer = Dryer::new(
        power,
        dotenv!("TARGET_TEMPERATURE").parse::<u8>()?,
        temp_sensor,
        fan
    );
    dryer.start()?;
    Ok(())
}
