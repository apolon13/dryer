use anyhow::anyhow;
use embedded_svc::mqtt::client::{EventPayload, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use std::thread;
use std::time::Duration;

pub struct Credentials {
    client_id: String,
    username: String,
    password: String,
    url: String,
}

impl Credentials {
    pub fn new(client_id: String, username: String, password: String, url: String) -> Self {
        Self {
            client_id,
            username,
            password,
            url,
        }
    }
}

pub struct Mqtt {
    credentials: Credentials,
}

impl Mqtt {
    pub fn new(credentials: Credentials) -> Self {
        Mqtt { credentials }
    }

    pub fn wait_message(&self) -> Result<(), anyhow::Error> {
        let mut client = EspMqttClient::new_cb(
            self.credentials.url.as_str(),
            &MqttClientConfiguration {
                client_id: Option::from(self.credentials.client_id.as_str()),
                username: Option::from(self.credentials.username.as_str()),
                password: Option::from(self.credentials.password.as_str()),
                keep_alive_interval: Option::from(Duration::from_secs(30)),
                ..MqttClientConfiguration::default()
            },
            move |message_event| match message_event.payload() {
                EventPayload::Received {
                    id,
                    topic,
                    data,
                    details,
                } => {
                    println!("received message: {:?}", &data);
                }
                _ => {}
            },
        )?;

        let mut subscribe_attempt = 0;
        let mut subscribed = false;
        loop {
            if !subscribed && subscribe_attempt < 100 {
                if subscribe_attempt < 50 {
                    match client.subscribe("run", QoS::AtMostOnce) {
                        Ok(_) => subscribed = true,
                        Err(_) => subscribe_attempt += 1,
                    };
                } else {
                    return Err(anyhow!("failed to subscribe to topic"));
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
