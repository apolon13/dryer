#![feature(mpmc_channel)]

#[macro_use]
mod schedule;
mod time;
mod wifi;
mod dryer;
mod mqtt;

use std::sync::{mpmc, mpsc};
use std::thread;
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
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::Resolution::Bits10;
use esp_idf_hal::units::Hertz;
use crate::dryer::fan::Fan;
use crate::dryer::heater::Heater;
use crate::mqtt::{Mqtt, Command};
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
    let (timers_tx, timers_rx) = mpsc::channel();
    let (cancel_tx, cancel_rx) = mpmc::channel();
    // Init WI-FI
    let sys_loop = EspSystemEventLoop::take()?;
    let wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), None)?;
    let mut connection = Connection::new(
        Credentials::new(
            dotenv!("SSID").to_string(),
            dotenv!("PASS").to_string(),
        ),
        wifi,
        sys_loop,
    );
    connection.open(AuthMethod::WPA2Personal)?;

    let handles = vec![
        thread::spawn(move || {
            // Init MQTT
            let mut mqtt = Mqtt::new(mqtt::Credentials::new(
                dotenv!("MQTT_CLIENT_ID").to_string(),
                dotenv!("MQTT_USERNAME").to_string(),
                dotenv!("MQTT_PASSWORD").to_string(),
                dotenv!("MQTT_URL").to_string(),
            )).unwrap();
            mqtt.wait(|mqtt: &mut Mqtt| {
                mqtt.on_command(|msg| {
                    match msg {
                        Command::Start(d) => {
                            Ok(timers_tx.send(SyncTimer::new(cancel_rx.clone(), d))?)
                        },
                        Command::Stop => {
                            Ok(cancel_tx.send(true)?)
                        },
                    }
                })?;
                mqtt.send_message()?;
                Ok(())
            }).unwrap();
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
