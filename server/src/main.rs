use std::collections::HashMap;
use std::time::Duration;
use std::fmt::Debug;

use sqlx::postgres::{PgPoolOptions, PgPool};
use serde::{Serialize, Deserialize};
use rocket::State;
use rocket_contrib::{json::Json, serve::StaticFiles};
use tokio::time;

type Result<T> = std::result::Result<T, rocket::response::Debug<sqlx::Error>>;

#[derive(Debug, Serialize, Deserialize)]
struct Measurement {
    timestamp: i64,
    health: f64,
    performance: f64,
    tvoc: f64,
    humidity: f64,
    humidity_abs: f64,
    temperature: f64,
    dewpt: f64,
    sound: f64,
    pressure: f64,
    no2: Option<f64>,
    co: Option<f64>,
    co2: f64,
    pm1: f64,
    pm2_5: f64,
    pm10: f64,
    oxygen: f64,
    o3: Option<f64>,
    so2: Option<f64>,
}

#[rocket::get("/data")]
async fn data(pool: State<'_, PgPool>) -> Result<Json<HashMap<&'static str, Vec<f64>>>> {
    let data = sqlx::query_as!(
            Measurement,
            r#"
                SELECT
                    timestamp, health, performance, tvoc, humidity,
                    humidity_abs, temperature, dewpt, sound, pressure,
                    no2, co, co2, pm1, pm2_5,
                    pm10, oxygen, o3, so2
                FROM measurements;
            "#,
        ).fetch_all(&*pool).await?;
    let mut map: HashMap<_, Vec<_>> = HashMap::new();
    for entry in data {
        map.entry("timestamp").or_default().push(entry.timestamp as f64);
        map.entry("health").or_default().push(entry.health);
        map.entry("performance").or_default().push(entry.performance);
        map.entry("tvoc").or_default().push(entry.tvoc);
        map.entry("humidity").or_default().push(entry.humidity);
        map.entry("humidity_abs").or_default().push(entry.humidity_abs);
        map.entry("temperature").or_default().push(entry.temperature);
        map.entry("dewpt").or_default().push(entry.dewpt);
        map.entry("sound").or_default().push(entry.sound);
        map.entry("pressure").or_default().push(entry.pressure);
        map.entry("no2").or_default().push(entry.no2.map(|no2| no2).unwrap_or_default());
        map.entry("co").or_default().push(entry.co.map(|co| co).unwrap_or_default());
        map.entry("co2").or_default().push(entry.co2);
        map.entry("pm1").or_default().push(entry.pm1);
        map.entry("pm2_5").or_default().push(entry.pm2_5);
        map.entry("pm10").or_default().push(entry.pm10);
        map.entry("oxygen").or_default().push(entry.oxygen);
        map.entry("o3").or_default().push(entry.o3.map(|o3| o3).unwrap_or_default());
        map.entry("so2").or_default().push(entry.so2.map(|so2| so2).unwrap_or_default());
    }
    Ok(Json(map))
}

async fn create_pool() -> Result<PgPool> {
    Ok(PgPoolOptions::new()
        .max_connections(8)
        .connect(env!("DATABASE_URL")).await?)
}

async fn fetch_data_regularly() {
    let mut interval = time::interval(Duration::from_secs(2 * 60));
    loop {
        interval.tick().await;
        match fetch_data::fetch_data().await {
            Ok(_) => (),
            Err(e) => eprintln!("Error fetching data from airQ: {:?}", e),
        }
    }
}

#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    tokio::spawn(fetch_data_regularly());

    rocket::ignite()
        .manage(create_pool().await.unwrap())
        .mount("/", StaticFiles::from("static/"))
        .mount("/", rocket::routes![data])
}
