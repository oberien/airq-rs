use sqlx::postgres::{PgPoolOptions, PgPool};
use rocket::State;
use rocket::response::Debug;

type Result<T> = std::result::Result<T, Debug<sqlx::Error>>;

#[rocket::get("/")]
async fn hello() -> &'static str {
    "uiae"
}

#[derive(Debug)]
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
async fn data(pool: State<'_, PgPool>) -> Result<String> {
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
    Ok(format!("{:#?}", data[0]))
}

async fn create_pool() -> Result<PgPool> {
    Ok(PgPoolOptions::new()
        .max_connections(8)
        .connect(env!("DATABASE_URL")).await?)
}

#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .manage(create_pool().await.unwrap())
        .mount("/", rocket::routes![hello, data])
}
