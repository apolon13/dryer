use std::sync::mpsc;
use anyhow::anyhow;
use embedded_svc::mqtt::client::{EventPayload, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use std::thread;
use std::time::Duration;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Message {
    command: String,
}

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

    pub fn wait_message<F: FnMut(Message)>(&self, mut cb: F) -> Result<(), anyhow::Error> {
        let (messages_tx, messages_rx) = mpsc::channel::<Message>();
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
                    let val: Message = serde_json::from_slice(data).unwrap();
                    messages_tx.send(val).unwrap();
                }
                _ => {}
            },
        )?;
        self.wait_subscribe(&mut client)?;
        for msg in messages_rx {
            cb(msg);
        }
        Ok(())
    }

    fn wait_subscribe(&self, client: &mut EspMqttClient) -> Result<(), anyhow::Error> {
        let mut subscribe_attempt = 0;
        let mut subscribed = false;
        loop {
            if !subscribed {
                if subscribe_attempt < 50 {
                    match client.subscribe("messages", QoS::AtMostOnce) {
                        Ok(_) => subscribed = true,
                        Err(_) => subscribe_attempt += 1,
                    };
                } else {
                    return Err(anyhow!("failed to subscribe to topic"));
                }
                thread::sleep(Duration::from_millis(50));
            } else {
                return Ok(())
            }
        }
    }
}
