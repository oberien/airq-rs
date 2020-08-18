use std::collections::HashMap;
use std::time::Duration;
use std::fmt::Debug;
use std::panic::AssertUnwindSafe;
use std::sync::Mutex;

use sqlx::postgres::{PgPoolOptions, PgPool};
use serde::{Serialize, Deserialize};
use rocket::State;
use rocket_contrib::{json::Json, serve::StaticFiles};
use tokio::time;
use futures::FutureExt;
use airq::Data14;
use fetch_data::FetchData;
use lazy_static::lazy_static;

type Result<T> = std::result::Result<T, rocket::response::Debug<sqlx::Error>>;

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
    Ok(Json(sqlx::query_as!(
        Timestamps,
        "SELECT min(timestamp) as first, max(timestamp) as last FROM measurements;"
    ).fetch_one(&*pool).await?))
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
async fn data(pool: State<'_, PgPool>, first: u64, last: u64) -> Result<Json<HashMap<&'static str, Vec<f64>>>> {
    let num_measurements = (last - first) / (2 * 60 * 1000);
    let combine_datapoints = (num_measurements + MAX_DATAPOINTS) / MAX_DATAPOINTS;
    let combine_millis = combine_datapoints * 2 * 60 * 1000;
    println!("{}, {}, {}", num_measurements, combine_datapoints, combine_millis);

    let data = sqlx::query_as!(
            Measurement,
            r#"
                SELECT
                    min(timestamp) as timestamp, avg(health) as health, avg(performance) as performance,
                    avg(tvoc) as tvoc, avg(humidity) as humidity, avg(humidity_abs) as humidity_abs,
                    avg(temperature) as temperature, avg(dewpt) as dewpt, avg(sound) as sound,
                    avg(pressure) as pressure, avg(no2) as no2, avg(co) as co,
                    avg(co2) as co2, avg(pm1) as pm1, avg(pm2_5) as pm2_5,
                    avg(pm10) as pm10, avg(oxygen) as oxygen, avg(o3) as o3,
                    avg(so2) as so2
                FROM measurements
                WHERE timestamp >= $1 AND timestamp <= $2
                GROUP BY timestamp / $3
                ORDER BY timestamp;
            "#,
            first as i64, last as i64, combine_millis as i64
        ).fetch_all(&*pool).await?;
    let mut map: HashMap<_, Vec<_>> = HashMap::new();
    for entry in data {
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

async fn create_pool() -> Result<PgPool> {
    Ok(PgPoolOptions::new()
        .max_connections(8)
        .connect(env!("DATABASE_URL")).await?)
}

async fn fetch_data_regularly() {
    let mut interval = time::interval(Duration::from_secs(1));
    let fetchdata = FetchData::new();
    loop {
        interval.tick().await;
        match AssertUnwindSafe(fetchdata.fetch_current()).catch_unwind().await {
            Ok(Err(e)) => eprintln!("Error fetching current data from airQ: {:?}", e),
            Ok(Ok(data)) => *CURRENT_DATA.lock().unwrap() = Some(data),
            Err(e) => eprintln!("Panic fetching current data from airQ: {:?}", e),
        }
    }
}
async fn fetch_current_data_regularly() {
    let mut interval = time::interval(Duration::from_secs(2 * 60));
    let fetchdata = FetchData::new();
    loop {
        interval.tick().await;
        match AssertUnwindSafe(fetchdata.fetch_data()).catch_unwind().await {
            Ok(Err(e)) => eprintln!("Error fetching data from airQ: {:?}", e),
            Ok(Ok(_)) => (),
            Err(e) => eprintln!("Panic fetching data from airQ: {:?}", e),
        }
    }
}

#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    tokio::spawn(fetch_data_regularly());
    tokio::spawn(fetch_current_data_regularly());

    rocket::ignite()
        .manage(create_pool().await.unwrap())
        .mount("/", StaticFiles::from("static/"))
        .mount("/", rocket::routes![timestamps, data_current, data])
}
