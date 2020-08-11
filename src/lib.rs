use std::time::{Instant, Duration};
use std::thread;
use std::marker::PhantomData;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::Value;
use reqwest::blocking::Client;
use block_modes::{BlockMode, Cbc, block_padding::Pkcs7};
use aes::Aes256;

mod error;
mod data;

pub use error::*;
pub use data::*;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

pub struct AirQ {
    key: [u8; 32],
    prefix: String,
    client: Client,
}

impl AirQ {
    pub fn new(domain: &str, password: &str) -> AirQ {
        let mut key = [b'0'; 32];
        let len = password.len().min(32);
        key[..len].copy_from_slice(&password.as_bytes()[..len]);
        AirQ {
            key,
            prefix: format!("http://{}", domain),
            client: Client::new(),
        }
    }

    fn request_raw(&self, path: &str) -> Result<String> {
        Ok(self.client.get(&format!("{}{}", self.prefix, path))
            .send()?
            .text()?)
    }
    fn request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        Ok(self.client.get(&format!("{}{}", self.prefix, path))
            .send()?
            .json()?)
    }

    pub fn blink(&self) -> Result<DeviceId> {
        self.request("/blink")
    }

    pub fn data_11(&self) -> Result<Data14> {
        self.data_raw()
    }
    pub fn data_14(&self) -> Result<Data14> {
        self.data_raw()
    }
    pub fn data_raw<T: DeserializeOwned>(&self) -> Result<T> {
        self.decrypt(&self.request::<Encrypted>("/data")?.content)
    }

    pub fn live_data_11(&self) -> Live<Data11> {
        self.live_data_raw()
    }
    pub fn live_data_14(&self) -> Live<Data14> {
        self.live_data_raw()
    }
    pub fn live_data_raw<T: DeserializeOwned>(&self) -> Live<T> {
        Live {
            last_request: Instant::now() - Duration::from_millis(2000),
            airq: &self,
            _marker: PhantomData,
        }
    }

    pub fn config(&self) -> Result<Value> {
        self.decrypt(&self.request::<Encrypted>("/config")?.content)
    }
    pub fn ping(&self) -> Result<Value> {
        let Encrypted { deviceid, content } = self.request("/ping")?;
        self.decrypt(&content)
    }
    pub fn standardpass(&self) -> Result<bool> {
        self.request("/standardpass")
    }
    pub fn dir(&self, path: &str) -> Result<Vec<String>> {
        self.decrypt(&self.request_raw(&format!("/dir?request={}", self.encrypt(path.as_bytes())?))?)
    }
    pub fn dirbuff(&self) -> Result<impl Iterator<Item = FilePath>> {
        let files: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> = self.decrypt(&self.request_raw("/dirbuff")?)?;
        Ok(files.into_iter().flat_map(|(year, months)| {
            let year = year.parse().unwrap();
            months.into_iter().flat_map(move |(month, days)| {
                let month = month.parse().unwrap();
                days.into_iter().flat_map(move |(day, timestamps)| {
                    let day = day.parse().unwrap();
                    timestamps.into_iter().map(move |timestamp| {
                        FilePath {
                            year,
                            month,
                            day,
                            timestamp: timestamp.parse().unwrap(),
                        }
                    })
                })
            })
        }))
    }
    pub fn file_data_11(&self, path: &str) -> Result<Vec<Data11>> {
        self.file_raw(path)
    }
    pub fn file_data_14(&self, path: &str) -> Result<Vec<Data14>> {
        self.file_raw(path)
    }
    pub fn file_raw<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let lines = self.request_raw(&format!("/file?request={}", self.encrypt(path.as_bytes())?))?;
        lines.lines()
            .filter(|line| !line.is_empty())
            .map(|line| self.decrypt(line))
            .collect()
    }
    pub fn file_recrypt_data_11(&self, path: &str) -> Result<Vec<Data11>> {
        self.file_recrypt_raw(path)
    }
    pub fn file_recrypt_data_14(&self, path: &str) -> Result<Vec<Data14>> {
        self.file_recrypt_raw(path)
    }
    pub fn file_recrypt_raw<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let lines = self.request_raw(&format!("/file_recrypt?request={}", self.encrypt(path.as_bytes())?))?;
        lines.lines()
            .filter(|line| !line.is_empty())
            .map(|line| self.decrypt(line))
            .collect()
    }
    pub fn log(&self) -> Result<Vec<String>> {
        self.decrypt(&self.request::<Encrypted>("/log")?.content)
    }

    fn decrypt<T: DeserializeOwned>(&self, encrypted: &str) -> Result<T> {
        let mut decoded = base64::decode(encrypted)?;
        let iv = &decoded[..16];
        let cipher = Aes256Cbc::new_var(&self.key, iv).unwrap();
        let ciphertext = &mut decoded[16..];
        let plaintext = cipher.decrypt(ciphertext)?;
        // println!("{}", std::str::from_utf8(plaintext).unwrap());
        Ok(serde_json::from_slice(plaintext)?)
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<String> {
        let iv: [u8; 16] = rand::random();
        let cipher = Aes256Cbc::new_var(&self.key, &iv).unwrap();
        let mut encrypted = vec![0; 16 + 16 * (plaintext.len() / 16 + 1)];
        encrypted[..16].copy_from_slice(&iv);
        encrypted[16..][..plaintext.len()].copy_from_slice(plaintext);
        let ciphertext = cipher.encrypt(&mut encrypted[16..], plaintext.len())?;
        assert_eq!(16 + ciphertext.len(), encrypted.len());
        Ok(base64::encode(encrypted))
    }
}

pub struct Live<'a, T: DeserializeOwned> {
    last_request: Instant,
    airq: &'a AirQ,
    _marker: PhantomData<T>,
}

impl<'a, T: DeserializeOwned> Iterator for Live<'a, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let elapsed = self.last_request.elapsed();
        if elapsed.as_millis() < 1500 {
            thread::sleep(Duration::from_millis(1500) - elapsed);
        }
        self.last_request = Instant::now();
        Some(self.airq.data_raw())
    }
}
