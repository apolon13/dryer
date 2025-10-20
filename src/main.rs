#![feature(mpmc_channel)]

#[macro_use]
mod schedule;
mod time;
mod wifi;
mod dryer;

use std::sync::{mpmc, mpsc};
use std::thread;
use std::time::Duration;
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
use esp_idf_hal::ledc::{LedcChannel, LedcDriver, LedcTimerDriver, Resolution};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::Resolution::Bits10;
use esp_idf_hal::units::Hertz;
use crate::dryer::fan::Fan;
use crate::dryer::heater::Heater;
use crate::time::timer::SyncTimer;

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
    let (timers_tx, timers_rx) = mpsc::channel::<SyncTimer>();
    let (cancel_tx, cancel_rx) = spmc::channel::<bool>();

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

    let handles = vec![
        thread::spawn(move || {
            loop {
                let timer = SyncTimer::new(cancel_rx.clone(), Duration::from_secs(5));
                timers_tx.send(timer).unwrap();
                thread::sleep(Duration::from_secs(10));
            }
        }),
        thread::spawn(move || {
            //Init fan
            let timer_driver = LedcTimerDriver::new(
                peripherals.ledc.timer0,
                &TimerConfig::default()
                    .frequency(Hertz::from(20_000u32))
                    .resolution(Bits10),
            ).unwrap();
            let pwm = LedcDriver::new(
                peripherals.ledc.channel0,
                timer_driver,
                peripherals.pins.gpio4,
            ).unwrap();
            let fan = Fan::new(pwm);

            //Init temperature sensor
            let mut pin_driver = PinDriver::output(peripherals.pins.gpio6).unwrap().into_input_output().unwrap();
            let wire = OneWire::new(&mut pin_driver, false);
            let temp_sensor = DS18B20Sensor::new(wire, 100).unwrap();

            //Init heater
            let power = PinDriver::output(peripherals.pins.gpio2).unwrap().into_output().unwrap();
            let mut dryer = Heater::new(
                power,
                dotenv!("TARGET_TEMPERATURE").parse::<u8>().unwrap(),
                temp_sensor,
                fan
            );
            for timer in timers_rx {
                dryer.start(timer).unwrap();
                dryer.stop().unwrap();
            }
        }),
    ];

    for handle in handles {
        handle.join().map_err(|e| anyhow::anyhow!("thread panicked: {:?}", e))?;
    }
    Ok(())
}
