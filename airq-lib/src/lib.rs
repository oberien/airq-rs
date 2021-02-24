#[cfg(feature = "blocking")]
use std::time::Instant;
use std::time::Duration;
#[cfg(feature = "blocking")]
use std::thread;
#[cfg(feature = "blocking")]
use std::marker::PhantomData;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::Value;
#[cfg(feature = "blocking")]
use reqwest::blocking::Client;
#[cfg(not(feature = "blocking"))]
use reqwest::Client;
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
}

impl AirQ {
    pub fn new(domain: &str, password: &str) -> AirQ {
        let mut key = [b'0'; 32];
        let len = password.len().min(32);
        key[..len].copy_from_slice(&password.as_bytes()[..len]);
        AirQ {
            key,
            prefix: format!("http://{}", domain),
        }
    }

    fn client(&self) -> Client {
        Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap()
    }

    #[cfg(not(feature = "blocking"))]
    async fn request_raw(&self, path: &str) -> Result<String> {
        Ok(self.client().get(&format!("{}{}", self.prefix, path))
            .send().await?
            .text().await?)
    }
    #[cfg(feature = "blocking")]
    fn request_raw(&self, path: &str) -> Result<String> {
        Ok(self.client().get(&format!("{}{}", self.prefix, path))
            .send()?
            .text()?)
    }
    #[cfg(not(feature = "blocking"))]
    async fn request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        println!("get client");
        let client = self.client().get(&format!("{}{}", self.prefix, path));
        println!("send request");
        let res = client.send().await?;
        println!("get json");
        let json = res.json().await?;
        println!("done");
        Ok(json)
    }
    #[cfg(feature = "blocking")]
    fn request<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        Ok(self.client().get(&format!("{}{}", self.prefix, path))
            .send()?
            .json()?)
    }

    #[cfg(not(feature = "blocking"))]
    pub async fn blink(&self) -> Result<DeviceId> {
        self.request("/blink").await
    }
    #[cfg(feature = "blocking")]
    pub fn blink(&self) -> Result<DeviceId> {
        self.request("/blink")
    }

    #[cfg(not(feature = "blocking"))]
    pub async fn data_11(&self) -> Result<Data14> {
        self.data_raw().await
    }
    #[cfg(feature = "blocking")]
    pub fn data_11(&self) -> Result<Data14> {
        self.data_raw()
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn data_14(&self) -> Result<Data14> {
        self.data_raw().await
    }
    #[cfg(feature = "blocking")]
    pub fn data_14(&self) -> Result<Data14> {
        self.data_raw()
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn data_raw<T: DeserializeOwned>(&self) -> Result<T> {
        self.decrypt(&self.request::<Encrypted>("/data").await?.content)
    }
    #[cfg(feature = "blocking")]
    pub fn data_raw<T: DeserializeOwned>(&self) -> Result<T> {
        self.decrypt(&self.request::<Encrypted>("/data")?.content)
    }

    #[cfg(feature = "blocking")]
    pub fn live_data_11(&self) -> Live<Data11> {
        self.live_data_raw()
    }
    #[cfg(feature = "blocking")]
    pub fn live_data_14(&self) -> Live<Data14> {
        self.live_data_raw()
    }
    #[cfg(feature = "blocking")]
    pub fn live_data_raw<T: DeserializeOwned>(&self) -> Live<T> {
        Live {
            last_request: Instant::now() - Duration::from_millis(2000),
            airq: &self,
            _marker: PhantomData,
        }
    }

    #[cfg(not(feature = "blocking"))]
    pub async fn config(&self) -> Result<Value> {
        self.decrypt(&self.request::<Encrypted>("/config").await?.content)
    }
    #[cfg(feature = "blocking")]
    pub fn config(&self) -> Result<Value> {
        self.decrypt(&self.request::<Encrypted>("/config")?.content)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn ping(&self) -> Result<Value> {
        let Encrypted { deviceid: _, content } = self.request("/ping").await?;
        self.decrypt(&content)
    }
    #[cfg(feature = "blocking")]
    pub fn ping(&self) -> Result<Value> {
        let Encrypted { deviceid: _, content } = self.request("/ping")?;
        self.decrypt(&content)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn standardpass(&self) -> Result<bool> {
        self.request("/standardpass").await
    }
    #[cfg(feature = "blocking")]
    pub fn standardpass(&self) -> Result<bool> {
        self.request("/standardpass")
    }

    #[cfg(not(feature = "blocking"))]
    pub async fn dir<S: AsRef<str>>(&self, path: S) -> Result<Vec<String>> {
        self.decrypt(&self.request_raw(&format!("/dir?request={}", self.encrypt(path.as_ref().as_bytes())?)).await?)
    }
    #[cfg(feature = "blocking")]
    pub fn dir<S: AsRef<str>>(&self, path: S) -> Result<Vec<String>> {
        self.decrypt(&self.request_raw(&format!("/dir?request={}", self.encrypt(path.as_ref():w.as_bytes())?))?)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn dirbuff(&self) -> Result<Vec<FilePath>> {
        let files: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> = self.decrypt(&self.request_raw("/dirbuff").await?)?;
        Ok(Self::aggregate_dirbuff(files))
    }
    #[cfg(feature = "blocking")]
    pub fn dirbuff(&self) -> Result<Vec<FilePath>> {
        let files: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> = self.decrypt(&self.request_raw("/dirbuff")?)?;
        Ok(Self::aggregate_dirbuff(files))
    }
    fn aggregate_dirbuff(files: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>) -> Vec<FilePath> {
        let mut files: Vec<_> = files.into_iter().flat_map(|(year, months)| {
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
        }).collect();
        files.sort();
       files
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn file_data_11(&self, path: &str) -> Result<Vec<Data11>> {
        self.file_raw(path).await
    }
    #[cfg(feature = "blocking")]
    pub fn file_data_11(&self, path: &str) -> Result<Vec<Data11>> {
        self.file_raw(path)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn file_data_14(&self, path: &str) -> Result<Vec<Data14>> {
        self.file_raw(path).await
    }
    #[cfg(feature = "blocking")]
    pub fn file_data_14(&self, path: &str) -> Result<Vec<Data14>> {
        self.file_raw(path)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn file_raw<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let lines = self.request_raw(&format!("/file?request={}", self.encrypt(path.as_bytes())?)).await?;
        self.aggregate_lines(lines)
    }
    #[cfg(feature = "blocking")]
    pub fn file_raw<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let lines = self.request_raw(&format!("/file?request={}", self.encrypt(path.as_bytes())?))?;
        self.aggregate_lines(lines)
    }
    fn aggregate_lines<T: DeserializeOwned>(&self, lines: String) -> Result<Vec<T>> {
        lines.lines()
            .filter(|line| !line.is_empty())
            .map(|line| self.decrypt(line))
            .collect()
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn file_recrypt_data_11(&self, path: &str) -> Result<Vec<Data11>> {
        self.file_recrypt_raw(path).await
    }
    #[cfg(feature = "blocking")]
    pub fn file_recrypt_data_11(&self, path: &str) -> Result<Vec<Data11>> {
        self.file_recrypt_raw(path)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn file_recrypt_data_14<S: AsRef<str>>(&self, path: S) -> Result<Vec<Data14>> {
        self.file_recrypt_raw(path.as_ref()).await
    }
    #[cfg(feature = "blocking")]
    pub fn file_recrypt_data_14<S: AsRef<str>>(&self, path: S) -> Result<Vec<Data14>> {
        self.file_recrypt_raw(path.as_ref())
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn file_recrypt_raw<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let lines = self.request_raw(&format!("/file_recrypt?request={}", self.encrypt(path.as_bytes())?)).await?;
        self.aggregate_lines(lines)
    }
    #[cfg(feature = "blocking")]
    pub fn file_recrypt_raw<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let lines = self.request_raw(&format!("/file_recrypt?request={}", self.encrypt(path.as_bytes())?))?;
        self.aggregate_lines(lines)
    }
    #[cfg(not(feature = "blocking"))]
    pub async fn log(&self) -> Result<Vec<String>> {
        self.decrypt(&self.request::<Encrypted>("/log").await?.content)
    }
    #[cfg(feature = "blocking")]
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

#[cfg(feature = "blocking")]
pub struct Live<'a, T: DeserializeOwned> {
    last_request: Instant,
    airq: &'a AirQ,
    _marker: PhantomData<T>,
}

#[cfg(feature = "blocking")]
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
