use embedded_svc::wifi::{AccessPointInfo, AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::IpEvent;
use esp_idf_svc::wifi::EspWifi;
use std::fmt::{Display, Formatter};
use std::time::Duration;

pub struct Credentials {
    ssid: String,
    password: String,
}

impl Display for Credentials {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("ssid", &self.ssid)
            .field("password", &self.password)
            .finish()
    }
}

impl Credentials {
    pub fn new(ssid: String, password: String) -> Credentials {
        Credentials { ssid, password }
    }
}

pub struct Connection<'a> {
    credentials: Credentials,
    wifi: EspWifi<'a>,
    event_loop: EspSystemEventLoop,
}

impl<'a> Connection<'a> {
    pub fn new(
        credentials: Credentials,
        wifi: EspWifi<'a>,
        event_loop: EspSystemEventLoop,
    ) -> Self {
        Connection {
            credentials,
            wifi,
            event_loop,
        }
    }

    pub fn open(&mut self, auth_method: AuthMethod) -> Result<(), anyhow::Error> {
        let Credentials { ssid, password } = &self.credentials;
        self.wifi.start()?;
        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.parse().unwrap(),
                password: password.parse().unwrap(),
                channel: None,
                auth_method,
                ..Default::default()
            }))?;
        self.connect()?;
        Ok(())
    }

    fn connect(&mut self) -> Result<(), anyhow::Error> {
        self.wifi.connect()?;
        let wait = esp_idf_svc::eventloop::Wait::new::<IpEvent>(&self.event_loop)?;
        wait.wait_while(
            || self.wifi.is_up().map(|s| !s),
            Option::from(Duration::from_secs(15)),
        )?;
        Ok(())
    }

    pub fn init_with_autoconfig(&mut self) -> Result<(), anyhow::Error> {
        let Credentials { ssid, password } = &self.credentials;
        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ..Default::default()
            }))?;
        self.wifi.start()?;
        let points = self.wifi.scan()?;
        let access_point = self.access_point_info(points)?;
        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.parse().unwrap(),
                password: password.parse().unwrap(),
                channel: Option::from(access_point.channel),
                auth_method: match password.is_empty() {
                    true => AuthMethod::None,
                    false => access_point.auth_method.unwrap(),
                },
                ..Default::default()
            }))?;
        self.connect()?;
        Ok(())
    }

    fn access_point_info(
        &self,
        points: Vec<AccessPointInfo>,
    ) -> Result<AccessPointInfo, anyhow::Error> {
        let access_point = points
            .into_iter()
            .find(|x| x.ssid.to_string() == self.credentials.ssid);
        match access_point {
            Some(ap) => Ok(ap),
            None => Err(anyhow::anyhow!("Access point not found")),
        }
    }
}
