use airq::{AirQ, Error, DecodeError};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};
use std::fs::File;

#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
struct Measurement {
    num: u64,
    measuretime: u64,
    timestamp: u64,
    elapsed: u64,
    response_time: u64,
    errors: u64,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let ip = AirQ::find_in_network();
    const CONCURRENT: usize = 3;
    const UPDATE: u64 = 50;

    let measurements: Vec<Measurement> = match File::open("measurements.json") {
        Ok(file) => serde_json::from_reader(file).unwrap(),
        Err(_) => Vec::new(),
    };
    let elapsed = measurements.last().map(|m| m.elapsed).unwrap_or_default();
    let elapsed = Duration::from_millis(elapsed);

    let errors = Arc::new(AtomicU64::new(measurements.last().map(|m| m.errors).unwrap_or_default()));
    let num = Arc::new(AtomicU64::new(measurements.last().map(|m| m.num).unwrap_or_default()));
    let measurements = Arc::new(Mutex::new(measurements));
    let airq = Arc::new(AirQ::new(&ip, &std::env::var("AIRQ_PASSWORD").unwrap()));
    let now = Arc::new(Instant::now() - elapsed);

    let mut handles = Vec::new();
    for _ in 0..CONCURRENT {
        let errors = Arc::clone(&errors);
        let num = Arc::clone(&num);
        let measurements = Arc::clone(&measurements);
        let airq = Arc::clone(&airq);
        let now = Arc::clone(&now);
        handles.push(tokio::spawn(async move {
            loop {
                let num = num.fetch_add(1, Ordering::SeqCst) + 1;
                let time = Instant::now();
                // let data = airq.data_14().await;
                // let log = airq.log().await;
                let data = airq.file_recrypt_data_14("2021/6/7/1623093314").await;
                let response_time = time.elapsed().as_millis() as u64;
                let measurement = match data {
                    Ok(_) => {
                        let errors = errors.load(Ordering::SeqCst);
                        Measurement {
                            num,
                            // measuretime: data.data11.measuretime as u64,
                            // timestamp: data.data11.timestamp,
                            measuretime: 0,
                            timestamp: 0,
                            elapsed: now.elapsed().as_millis() as u64,
                            response_time,
                            errors,
                        }
                    }
                    Err(Error::Base64Error(DecodeError::InvalidByte(index, b))) if index > 5000 && b == 61 => {
                        let errors = errors.load(Ordering::SeqCst);
                        Measurement {
                            num,
                            // measuretime: data.data11.measuretime as u64,
                            // timestamp: data.data11.timestamp,
                            measuretime: 0,
                            timestamp: 0,
                            elapsed: now.elapsed().as_millis() as u64,
                            response_time,
                            errors,
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                        let errors = errors.fetch_add(1, Ordering::SeqCst) + 1;
                        Measurement {
                            num,
                            measuretime: 0,
                            timestamp: 0,
                            elapsed: now.elapsed().as_millis() as u64,
                            response_time,
                            errors,
                        }
                    }
                };
                let mut measurements = measurements.lock().await;
                measurements.push(measurement);
                if num % UPDATE == 0 {
                    let from = if measurements.len() < UPDATE as usize { 0 } else { measurements.len() - UPDATE as usize };
                    measurements[from..].sort_unstable();
                    let data = serde_json::to_vec(&*measurements).unwrap();
                    tokio::fs::write("measurements.json", &data).await.unwrap();
                    println!("written");
                }
                println!("done");
            }
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }
}
