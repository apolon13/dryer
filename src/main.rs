#[macro_use]
mod schedule;
mod time;
mod wifi;
mod dryer;
mod mqtt;

use std::thread;
use std::time::{Duration};
use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::wifi::EspWifi;
use log::error;
use onewire::OneWire;
use dryer::sensor::temperature::DS18B20Sensor;
use wifi::{Connection, Credentials};
use dotenv_codegen::dotenv;
use embedded_svc::wifi::AuthMethod;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::Resolution::Bits10;
use esp_idf_hal::units::Hertz;
use dryer::fan::Fan;
use dryer::heater::Heater;
use dryer::{State, StateMessage};
use mqtt::{Mqtt, Command};
use time::limit::OnceIn;
use time::timer::SyncTimer;
use crossbeam_channel::{unbounded};

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
    let (timers_tx, timers_rx) = unbounded();
    let (cancel_tx, cancel_rx) = unbounded();
    let (states_tx, states_rx) = unbounded();
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
            let mut state_limiter = OnceIn::new(Duration::from_secs(10));
            // Init MQTT
            let mut mqtt = Mqtt::new(mqtt::Credentials::new(
                dotenv!("MQTT_CLIENT_ID").to_string(),
                dotenv!("MQTT_USERNAME").to_string(),
                dotenv!("MQTT_PASSWORD").to_string(),
                dotenv!("MQTT_URL").to_string(),
            )).unwrap();

            mqtt.send_message(StateMessage::new(State::inactive())).unwrap();
            mqtt.wait(|mqtt| {
                let send_state = |mqtt: &mut Mqtt, state: State| -> Result<(), anyhow::Error>{
                    mqtt.send_message(StateMessage::new(state))
                };
                mqtt.on_command(|mqtt, msg| {
                    match msg {
                        Command::Start(d) => {
                            send_state(mqtt, State::active())?;
                            Ok(timers_tx.send(SyncTimer::new(cancel_rx.clone(), d))?)
                        },
                        Command::Stop => {
                            send_state(mqtt, State::inactive())?;
                            Ok(cancel_tx.send(true)?)
                        },
                    }
                })?;
                if let Ok(state) = states_rx.try_recv() {
                    state_limiter.if_allow(|| {
                        send_state(mqtt, state)
                    })?;
                }
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
                dotenv!("TARGET_TEMPERATURE").parse::<u16>().unwrap(),
                temp_sensor,
                fan
            );

            for timer in timers_rx {
                dryer.start(timer, states_tx.clone()).unwrap();
                states_tx.try_send(State::inactive()).unwrap();
                dryer.stop().unwrap();
            }
        }),
    ];

    for handle in handles {
        handle.join().map_err(|e| anyhow::anyhow!("thread panicked: {:?}", e))?;
    }
    Ok(())
}
