pub mod model;

use chrono::{NaiveDateTime, ParseResult};
use core::str;
use embedded_svc::http::client::Client;
use embedded_svc::http::Method;
use embedded_svc::utils::io;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use serde::de::DeserializeOwned;
use serde_json;


pub struct Request {
    client: Client<EspHttpConnection>,
}

impl Request {
    pub fn new_https() -> Result<Self, anyhow::Error> {
        let client = Client::wrap(EspHttpConnection::new(&Configuration {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        })?);
        Ok(Request { client })
    }

    pub fn time<T: DeserializeOwned>(
        &mut self,
        url: &str,
        headers: Vec<(&str, &str)>,
        f: fn(T) -> ParseResult<NaiveDateTime>,
    ) -> Result<NaiveDateTime, anyhow::Error> {
        let req = self.client.request(Method::Get, url.as_ref(), &headers)?;
        let mut result = req.submit()?;
        let mut buf = [0u8; 1024];
        let bytes_read = io::try_read_full(&mut result, &mut buf).map_err(|e| e.0)?;
        match str::from_utf8(&buf[0..bytes_read]) {
            Ok(body_string) => {
                let val: T = serde_json::from_str(body_string)?;
                let t = f(val)?;
                Ok(t)
            }
            Err(e) => Err(e.into()),
        }
    }
}
