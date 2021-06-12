use std::collections::HashMap;
use std::time::Duration;
use std::fmt::Debug;
use std::panic::AssertUnwindSafe;
use std::sync::Mutex;
use std::fs::File;

use sqlx::postgres::{PgPoolOptions, PgPool};
use serde::{Serialize, Deserialize};
use rocket::{State, Config, Request, Data, Route};
use rocket::figment::Figment;
use rocket_contrib::{json::Json, serve::StaticFiles};
use tokio::time;
use futures::FutureExt;
use airq::{Data14, AirQ, FilePath, Data11};
use lazy_static::lazy_static;

type Result<T> = std::result::Result<T, rocket::response::Debug<sqlx::Error>>;

mod fetch_data;
#[cfg(not(debug_assertions))]
mod include_static_files;

use fetch_data::FetchData;
#[cfg(not(debug_assertions))]
use include_static_files::IncludedStaticFiles;

#[derive(Debug, Serialize, Deserialize)]
struct Measurement {
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
struct Timestamps {
    first: Option<i64>,
    last: Option<i64>,
}

#[rocket::get("/timestamps")]
async fn timestamps(pool: State<'_, PgPool>) -> Result<Json<Timestamps>> {
    let mut seven_days = SEVEN_DAYS.lock().unwrap();
    let first = seven_days.data.first().map(|data| data.data11.timestamp as i64);
    let last = seven_days.data.last().map(|data| data.data11.timestamp as i64);
    Ok(Json(Timestamps { first, last }))
}

// #[rocket::get("/timestamps")]
// async fn timestamps(pool: State<'_, PgPool>) -> Result<Json<Timestamps>> {
//     Ok(Json(sqlx::query_as!(
//         Timestamps,
//         "SELECT min(timestamp) as first, max(timestamp) as last FROM measurements;"
//     ).fetch_one(&*pool).await?))
// }

const MAX_DATAPOINTS: u64 = 500;

lazy_static! {
    static ref CURRENT_DATA: Mutex<Option<Data14>> = Mutex::new(None);
    static ref SEVEN_DAYS: Mutex<SevenDays> = Mutex::new(SevenDays::default());
}

#[derive(Default, Serialize, Deserialize)]
struct SevenDays {
    last_file: Option<FilePath>,
    data: Vec<Data14>,
}

#[rocket::get("/data/current")]
async fn data_current() -> Json<Option<Data14>> {
    Json(CURRENT_DATA.lock().unwrap().clone())
}

#[rocket::get("/data/<first>/<last>")]
async fn data(pool: State<'_, PgPool>, first: u64, last: u64) -> Result<Json<HashMap<&'static str, Vec<f64>>>> {
    let num_measurements = (last - first) / (2 * 60 * 1000);
    let combine_datapoints = (num_measurements + MAX_DATAPOINTS) / MAX_DATAPOINTS;
    let combine_millis = combine_datapoints * 2 * 60 * 1000;
    println!("{}, {}, {}", num_measurements, combine_datapoints, combine_millis);

    struct MyData {
        timestamp: f64,
        health: f64,
        performance: f64,
        tvoc: f64,
        humidity: f64,
        humidity_abs: f64,
        temperature: f64,
        dewpt: f64,
        sound: f64,
        pressure: f64,
        no2: f64,
        co: f64,
        co2: f64,
        pm1: f64,
        pm2_5: f64,
        pm10: f64,
        oxygen: f64,
        o3: f64,
        so2: f64,
    }

    let mut seven_days = SEVEN_DAYS.lock().unwrap();
    let data = seven_days.data.chunks(combine_datapoints as usize)
        .map(|data| {
            fn avg(data: &[Data14], f: impl Fn(&Data14) -> f64) -> f64 {
                data.iter().map(f).sum::<f64>() / data.len() as f64
            }
            MyData {
                timestamp: data[0].data11.timestamp as f64,
                health: avg(data, |entry| entry.data11.health),
                performance: avg(data, |entry| entry.data11.performance),
                tvoc: avg(data, |entry| entry.data11.tvoc[0]),
                humidity: avg(data, |entry| entry.data11.humidity[0]),
                humidity_abs: avg(data, |entry| entry.data11.humidity_abs[0]),
                temperature: avg(data, |entry| entry.data11.temperature[0]),
                dewpt: avg(data, |entry| entry.data11.dewpt[0]),
                sound: avg(data, |entry| entry.data11.sound[0]),
                pressure: avg(data, |entry| entry.data11.pressure[0]),
                no2: avg(data, |entry| entry.data11.no2.unwrap_or_default()[0]),
                co: avg(data, |entry| entry.data11.co.unwrap_or_default()[0]),
                co2: avg(data, |entry| entry.data11.co2[0]),
                pm1: avg(data, |entry| entry.data11.pm1[0]),
                pm2_5: avg(data, |entry| entry.data11.pm2_5[0]),
                pm10: avg(data, |entry| entry.data11.pm10[0]),
                oxygen: avg(data, |entry| entry.oxygen[0]),
                o3: avg(data, |entry| entry.o3.unwrap_or_default()[0]),
                so2: avg(data, |entry| entry.so2.unwrap_or_default()[0]),
            }
        })
        .skip_while(|data| data.timestamp < first as f64)
        .take_while(|data| data.timestamp <= last as f64);

    let mut map: HashMap<_, Vec<_>> = HashMap::new();
    for data in data {
        map.entry("timestamp").or_default().push(data.timestamp);
        map.entry("health").or_default().push(data.health);
        map.entry("performance").or_default().push(data.performance);
        map.entry("tvoc").or_default().push(data.tvoc);
        map.entry("humidity").or_default().push(data.humidity);
        map.entry("humidity_abs").or_default().push(data.humidity_abs);
        map.entry("temperature").or_default().push(data.temperature);
        map.entry("dewpt").or_default().push(data.dewpt);
        map.entry("sound").or_default().push(data.sound);
        map.entry("pressure").or_default().push(data.pressure);
        map.entry("no2").or_default().push(data.no2);
        map.entry("co").or_default().push(data.co);
        map.entry("co2").or_default().push(data.co2);
        map.entry("pm1").or_default().push(data.pm1);
        map.entry("pm2_5").or_default().push(data.pm2_5);
        map.entry("pm10").or_default().push(data.pm10);
        map.entry("oxygen").or_default().push(data.oxygen);
        map.entry("o3").or_default().push(data.o3);
        map.entry("so2").or_default().push(data.so2);
    }
    Ok(Json(map))
}
// #[rocket::get("/data/<first>/<last>")]
// async fn data(pool: State<'_, PgPool>, first: u64, last: u64) -> Result<Json<HashMap<&'static str, Vec<f64>>>> {
//     let num_measurements = (last - first) / (2 * 60 * 1000);
//     let combine_datapoints = (num_measurements + MAX_DATAPOINTS) / MAX_DATAPOINTS;
//     let combine_millis = combine_datapoints * 2 * 60 * 1000;
//     println!("{}, {}, {}", num_measurements, combine_datapoints, combine_millis);
//
//     let data = sqlx::query_as!(
//             Measurement,
//             r#"
//                 SELECT
//                     min(timestamp) as timestamp, avg(health) as health, avg(performance) as performance,
//                     avg(tvoc) as tvoc, avg(humidity) as humidity, avg(humidity_abs) as humidity_abs,
//                     avg(temperature) as temperature, avg(dewpt) as dewpt, avg(sound) as sound,
//                     avg(pressure) as pressure, avg(no2) as no2, avg(co) as co,
//                     avg(co2) as co2, avg(pm1) as pm1, avg(pm2_5) as pm2_5,
//                     avg(pm10) as pm10, avg(oxygen) as oxygen, avg(o3) as o3,
//                     avg(so2) as so2
//                 FROM measurements
//                 WHERE timestamp >= $1 AND timestamp <= $2
//                 GROUP BY timestamp / $3
//                 ORDER BY timestamp;
//             "#,
//             first as i64, last as i64, combine_millis as i64
//         ).fetch_all(&*pool).await?;
//     let mut map: HashMap<_, Vec<_>> = HashMap::new();
//     for entry in data {
//         map.entry("timestamp").or_default().push(entry.timestamp.unwrap_or_default() as f64);
//         map.entry("health").or_default().push(entry.health.unwrap_or_default());
//         map.entry("performance").or_default().push(entry.performance.unwrap_or_default());
//         map.entry("tvoc").or_default().push(entry.tvoc.unwrap_or_default());
//         map.entry("humidity").or_default().push(entry.humidity.unwrap_or_default());
//         map.entry("humidity_abs").or_default().push(entry.humidity_abs.unwrap_or_default());
//         map.entry("temperature").or_default().push(entry.temperature.unwrap_or_default());
//         map.entry("dewpt").or_default().push(entry.dewpt.unwrap_or_default());
//         map.entry("sound").or_default().push(entry.sound.unwrap_or_default());
//         map.entry("pressure").or_default().push(entry.pressure.unwrap_or_default());
//         map.entry("no2").or_default().push(entry.no2.unwrap_or_default());
//         map.entry("co").or_default().push(entry.co.unwrap_or_default());
//         map.entry("co2").or_default().push(entry.co2.unwrap_or_default());
//         map.entry("pm1").or_default().push(entry.pm1.unwrap_or_default());
//         map.entry("pm2_5").or_default().push(entry.pm2_5.unwrap_or_default());
//         map.entry("pm10").or_default().push(entry.pm10.unwrap_or_default());
//         map.entry("oxygen").or_default().push(entry.oxygen.unwrap_or_default());
//         map.entry("o3").or_default().push(entry.o3.unwrap_or_default());
//         map.entry("so2").or_default().push(entry.so2.unwrap_or_default());
//     }
//     Ok(Json(map))
// }

async fn create_pool() -> Result<PgPool> {
    Ok(PgPoolOptions::new()
        .max_connections(8)
        .connect(&std::env::var("DATABASE_URL").unwrap()).await?)
}

async fn fetch_current_data_regularly(airq_ip: String, password: String) {
    let mut interval = time::interval(Duration::from_secs(5));
    let fetchdata = FetchData::new(&airq_ip, &password);
    loop {
        interval.tick().await;
        match AssertUnwindSafe(fetchdata.fetch_current()).catch_unwind().await {
            Ok(Err(e)) => eprintln!("Error fetching current data from airQ: {:?}", e),
            Ok(Ok(data)) => *CURRENT_DATA.lock().unwrap() = Some(data),
            Err(e) => eprintln!("Panic fetching current data from airQ: {:?}", e),
        }
    }
}
async fn fetch_data_regularly(airq_ip: String, password: String, pg_pool: PgPool) {
    let mut interval = time::interval(Duration::from_secs(2 * 60));
    let fetchdata = FetchData::new(&airq_ip, &password);
    loop {
        interval.tick().await;
        // match AssertUnwindSafe(fetchdata.fetch_data(&pg_pool)).catch_unwind().await {
        let last_file = SEVEN_DAYS.lock().unwrap().last_file.clone();
        let last_timestamp = SEVEN_DAYS.lock().unwrap().data.last().map(|data| data.data11.timestamp);
        match AssertUnwindSafe(fetchdata.fetch_data(last_file, last_timestamp)).catch_unwind().await {
            Ok(Err(e)) => eprintln!("Error fetching data from airQ: {:?}", e),
            Ok(Ok((last_file, data))) => {
                let mut seven_days = SEVEN_DAYS.lock().unwrap();
                let last_ts = seven_days.data.last()
                    .map(|data| data.data11.timestamp)
                    .unwrap_or_default();
                let to_add = data.into_iter().skip_while(|data| data.data11.timestamp <= last_ts);
                seven_days.data.extend(to_add);
                seven_days.last_file = Some(last_file);
                seven_days.data.sort_unstable_by_key(|data| data.data11.timestamp);
                serde_json::to_writer(File::create("sevendays.json").unwrap(), &*seven_days).unwrap();
            },
            Err(e) => eprintln!("Panic fetching data from airQ: {:?}", e),
        }
    }
}


#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    if cfg!(debug_assertions) {
        let _ = dotenv::dotenv();
    }
    match File::open("sevendays.json") {
        Ok(file) => *SEVEN_DAYS.lock().unwrap() = serde_json::from_reader(file).unwrap(),
        Err(_) => (),
    }

    let pool = create_pool().await.unwrap();

    let airq_ip = AirQ::find_in_network();
    println!("Using AirQ at {}", airq_ip);
    let password = std::env::var("AIRQ_PASSWORD").unwrap();

    tokio::spawn(fetch_current_data_regularly(airq_ip.clone(), password.clone()));
    tokio::spawn(fetch_data_regularly(airq_ip, password, pool.clone()));

    let mut config = Config::default();
    if config.port == 8000 {
        config.port = 8080;
    }
    let rocket = rocket::custom(Figment::from(config))
        .manage(pool);
    #[cfg(debug_assertions)]
    let rocket = rocket.mount("/", StaticFiles::from("static/"));
    #[cfg(not(debug_assertions))]
    let rocket = rocket.mount("/", IncludedStaticFiles);
    rocket.mount("/", rocket::routes![timestamps, data_current, data])
}
