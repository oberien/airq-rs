use std::collections::HashMap;
use std::time::Duration;
use std::fmt::Debug;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};

use serde::{Serialize, Deserialize};
use rocket::State;
use rocket_contrib::{json::Json, serve::StaticFiles};
use tokio::time;
use futures::FutureExt;
use airq::{Data14, AirQ};
use lazy_static::lazy_static;

type Result<T> = std::result::Result<T, rocket::response::Debug<Error>>;

mod fetch_data;
mod include_static_files;
mod storage;

use fetch_data::FetchData;
use include_static_files::IncludedStaticFiles;
use crate::fetch_data::Error;
use crate::storage::{MeasurementStorage, Postgres, Sevendays};

#[derive(Debug, Serialize, Deserialize)]
pub struct Measurement {
    timestamp: Option<i64>,
    health: Option<f64>,
    performance: Option<f64>,
    tvoc: Option<f64>,
    humidity: Option<f64>,
    humidity_abs: Option<f64>,
    temperature: Option<f64>,
    dewpt: Option<f64>,
    sound: Option<f64>,
    pressure: Option<f64>,
    no2: Option<f64>,
    co: Option<f64>,
    co2: Option<f64>,
    pm1: Option<f64>,
    pm2_5: Option<f64>,
    pm10: Option<f64>,
    oxygen: Option<f64>,
    o3: Option<f64>,
    so2: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct Timestamps {
    first: Option<i64>,
    last: Option<i64>,
}

#[rocket::get("/timestamps")]
async fn timestamps(storage: State<'_, Arc<dyn MeasurementStorage>>) -> Result<Json<Timestamps>> {
    Ok(Json(storage.timestamps().await?))
}

const MAX_DATAPOINTS: u64 = 500;

lazy_static! {
    static ref CURRENT_DATA: Mutex<Option<Data14>> = Mutex::new(None);
}


#[rocket::get("/data/current")]
async fn data_current() -> Json<Option<Data14>> {
    Json(CURRENT_DATA.lock().unwrap().clone())
}

#[rocket::get("/data/<first>/<last>")]
async fn data(storage: State<'_, Arc<dyn MeasurementStorage>>, first: u64, last: u64) -> Result<Json<HashMap<&'static str, Vec<f64>>>> {
    let num_measurements = (last - first) / (2 * 60 * 1000);
    let combine_datapoints = (num_measurements + MAX_DATAPOINTS) / MAX_DATAPOINTS;
    let combine_millis = combine_datapoints * 2 * 60 * 1000;
    println!("getting data for {}, {}, {}", num_measurements, combine_datapoints, combine_millis);

    let measurements = storage.data(first, last, combine_datapoints, combine_millis).await?;

    let mut map: HashMap<_, Vec<_>> = HashMap::new();
    for entry in measurements {
        map.entry("timestamp").or_default().push(entry.timestamp.unwrap_or_default() as f64);
        map.entry("health").or_default().push(entry.health.unwrap_or_default());
        map.entry("performance").or_default().push(entry.performance.unwrap_or_default());
        map.entry("tvoc").or_default().push(entry.tvoc.unwrap_or_default());
        map.entry("humidity").or_default().push(entry.humidity.unwrap_or_default());
        map.entry("humidity_abs").or_default().push(entry.humidity_abs.unwrap_or_default());
        map.entry("temperature").or_default().push(entry.temperature.unwrap_or_default());
        map.entry("dewpt").or_default().push(entry.dewpt.unwrap_or_default());
        map.entry("sound").or_default().push(entry.sound.unwrap_or_default());
        map.entry("pressure").or_default().push(entry.pressure.unwrap_or_default());
        map.entry("no2").or_default().push(entry.no2.unwrap_or_default());
        map.entry("co").or_default().push(entry.co.unwrap_or_default());
        map.entry("co2").or_default().push(entry.co2.unwrap_or_default());
        map.entry("pm1").or_default().push(entry.pm1.unwrap_or_default());
        map.entry("pm2_5").or_default().push(entry.pm2_5.unwrap_or_default());
        map.entry("pm10").or_default().push(entry.pm10.unwrap_or_default());
        map.entry("oxygen").or_default().push(entry.oxygen.unwrap_or_default());
        map.entry("o3").or_default().push(entry.o3.unwrap_or_default());
        map.entry("so2").or_default().push(entry.so2.unwrap_or_default());
    }
    Ok(Json(map))
}

async fn fetch_current_data_regularly(airq_ip: String, password: String, storage: Arc<dyn MeasurementStorage>) {
    let fetchdata = FetchData::new(&airq_ip, &password);
    loop {
        match AssertUnwindSafe(fetchdata.fetch_current()).catch_unwind().await {
            Ok(Err(e)) => eprintln!("Error fetching current data from airQ: {:?}", e),
            Ok(Ok(data)) => {
                match storage.store_manual_readout(&data).await {
                    Ok(()) => (),
                    Err(e) => eprintln!("couldn't store manual readout: {e:?}"),
                }
                *CURRENT_DATA.lock().unwrap() = Some(data);
            },
            Err(e) => eprintln!("Panic fetching current data from airQ: {:?}", e),
        }
        time::sleep(Duration::from_secs(5)).await;
    }
}
async fn fetch_data_regularly(airq_ip: String, password: String, storage: Arc<dyn MeasurementStorage>) {
    let fetchdata = FetchData::new(&airq_ip, &password);
    loop {
        match AssertUnwindSafe(fetchdata.fetch_data(&*storage)).catch_unwind().await {
            Ok(Err(e)) => eprintln!("Error fetching data from airQ: {:?}", e),
            Ok(Ok(())) => (),
            Err(e) => eprintln!("Panic fetching data from airQ: {:?}", e),
        }
        time::sleep(Duration::from_secs(2 * 60)).await;
    }
}
async fn clean_manual_readouts_regularly(storage: Arc<dyn MeasurementStorage>) {
    loop {
        match storage.clean_manual_readouts().await {
            Ok(()) => (),
            Err(e) => eprintln!("cleaning manual readouts failed: {:?}", e),
        }
        // once per day
        time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    if cfg!(debug_assertions) {
        let _ = dotenv::dotenv();
    }
    let storage: Arc<dyn MeasurementStorage> = if cfg!(debug_assertions) {
        Arc::new(Sevendays::open())
    } else {
        Arc::new(Postgres::connect().await)
    };

    let airq_ip = AirQ::find_in_network();
    println!("Using AirQ at {}", airq_ip);
    let password = std::env::var("AIRQ_PASSWORD").unwrap();

    tokio::spawn(fetch_current_data_regularly(airq_ip.clone(), password.clone(), Arc::clone(&storage)));
    tokio::spawn(fetch_data_regularly(airq_ip, password, Arc::clone(&storage)));
    tokio::spawn(clean_manual_readouts_regularly(Arc::clone(&storage)));

    let rocket = rocket::ignite()
        .manage(storage);
    // static files
    let rocket = if cfg!(debug_assertions) {
        rocket.mount("/", StaticFiles::from("static/"))
    } else {
        rocket.mount("/", IncludedStaticFiles)
    };
    // routes
    rocket.mount("/", rocket::routes![timestamps, data_current, data])
}
