use anyhow::anyhow;
use embedded_svc::mqtt::client::{EventPayload, MessageId, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use serde::Deserialize;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum Command {
    Start(Duration),
    Stop,
}

#[derive(Debug, Deserialize)]
struct StartOptions {
    duration: u64,
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

pub struct Mqtt<'a> {
    messages_rx: mpsc::Receiver<Command>,
    client: EspMqttClient<'a>,
}

impl Mqtt<'_> {
    pub fn new(credentials: Credentials) -> Result<Self, anyhow::Error> {
        let (messages_tx, messages_rx) = mpsc::channel::<Command>();
        let tx_cb = messages_tx.clone();
        let client = EspMqttClient::new_cb(
            credentials.url.as_str(),
            &MqttClientConfiguration {
                client_id: Option::from(credentials.client_id.as_str()),
                username: Option::from(credentials.username.as_str()),
                password: Option::from(credentials.password.as_str()),
                ..MqttClientConfiguration::default()
            },
            move |message_event| match message_event.payload() {
                EventPayload::Received {
                    id,
                    topic,
                    data,
                    details,
                } => match topic {
                    Some("/start") => {
                        let val: StartOptions = serde_json::from_slice(data).unwrap();
                        tx_cb
                            .send(Command::Start(Duration::from_secs(val.duration)))
                            .unwrap();
                    }
                    Some("/stop") => {
                        tx_cb.send(Command::Stop).unwrap();
                    }
                    c => {
                        println!("{:?}", c);
                    }
                },
                _ => {}
            },
        )?;
        Ok(Mqtt {
            messages_rx,
            client,
        })
    }

    pub fn wait<F: FnMut(&mut Self) -> Result<(), anyhow::Error>>(
        &mut self,
        mut cb: F,
    ) -> Result<(), anyhow::Error> {
        loop {
            cb(self)?;
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn on_command<F: FnMut(&mut Self, Command) -> Result<(), anyhow::Error>>(
        &mut self,
        mut cb: F,
    ) -> Result<(), anyhow::Error> {
        match self.messages_rx.try_recv() {
            Ok(cmd) => cb(self, cmd)?,
            _ => {}
        };
        Ok(())
    }

    pub fn send_message<M: MqttMessage>(&mut self, m: M) -> Result<(), anyhow::Error> {
        self.client.publish(m.topic(), QoS::AtLeastOnce, false, m.to_bytes())?;
        Ok(())
    }
}

pub trait MqttMessage {
    fn to_bytes(&self) -> &[u8];

    fn topic(&self) -> &str;
}
