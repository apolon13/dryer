use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use anyhow::anyhow;
use embedded_svc::mqtt::client::{EventPayload, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use serde::Deserialize;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum MessageType {
    Command(Command),
}

#[derive(Debug, Deserialize)]
pub struct Command {
    name: String,
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

    pub fn wait_message<F: FnMut(MessageType)>(&self, mut cb: F) -> Result<(), anyhow::Error> {
        let (messages_tx, messages_rx) = mpsc::channel::<MessageType>();
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
                    match topic {
                        Some("/commands") => {
                            let val: Command = serde_json::from_slice(data).unwrap();
                            messages_tx.send(MessageType::Command(val)).unwrap();
                        }
                        c => {
                            println!("{:?}", c);
                        }
                    }
                }
                _ => {}
            },
        )?;
        self.wait_subscription(|| match client.subscribe("commands", QoS::AtMostOnce) {
            Ok(_) => Ok(()),
            Err(error) => Err(anyhow!("subscribe to messages: {:?}", error)),
        })?;
        for msg in messages_rx {
            cb(msg)
        }
        Ok(())
    }

    fn wait_subscription<F: FnMut() -> Result<(), anyhow::Error>>(
        &self,
        mut sb: F,
    ) -> Result<(), anyhow::Error> {
        let mut subscribe_attempt = 0;
        let mut subscribed = false;
        loop {
            if !subscribed {
                match sb() {
                    Ok(_) => subscribed = true,
                    Err(_) => {
                        if subscribe_attempt < 50 {
                            subscribe_attempt += 1;
                        } else {
                            return Err(anyhow!("failed to subscribe to topic"));
                        }
                    }
                };
                thread::sleep(Duration::from_millis(50));
            } else {
                return Ok(());
            }
        }
    }
}
