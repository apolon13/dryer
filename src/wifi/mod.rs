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

pub struct Connection {
    credentials: Credentials
}

impl Connection {
    pub fn new(credentials: Credentials) -> Self {
        Connection { credentials}
    }

    pub fn open(&self, wifi: &mut EspWifi, event_loop: EspSystemEventLoop, auth_method: AuthMethod) -> Result<(), anyhow::Error> {
        wifi.start()?;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: self.credentials.ssid.parse().unwrap(),
                password: self.credentials.password.parse().unwrap(),
                channel: None,
                auth_method,
                ..Default::default()
            }))?;
        self.wait_connection(wifi, event_loop)?;
        Ok(())
    }

    fn wait_connection(&self, wifi: &mut EspWifi, event_loop: EspSystemEventLoop) -> Result<(), anyhow::Error> {
        wifi.connect()?;
        let wait = esp_idf_svc::eventloop::Wait::new::<IpEvent>(&event_loop)?;
        wait.wait_while(
            || wifi.is_up().map(|s| !s),
            Option::from(Duration::from_secs(15)),
        )?;
        Ok(())
    }

    pub fn open_with_autoconfig(&self, wifi: &mut EspWifi, event_loop: EspSystemEventLoop) -> Result<(), anyhow::Error> {
        let Credentials { ssid, password } = &self.credentials;
        wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ..Default::default()
            }))?;
        wifi.start()?;
        let points = wifi.scan()?;
        let access_point = self.access_point_info(points)?;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.parse().unwrap(),
                password: password.parse().unwrap(),
                channel: Option::from(access_point.channel),
                auth_method: match password.is_empty() {
                    true => AuthMethod::None,
                    false => access_point.auth_method.unwrap(),
                },
                ..Default::default()
            }))?;
        self.wait_connection(wifi, event_loop)?;
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
