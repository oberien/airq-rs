use std::fs;

use airq::{AirQ, Data11, Data14, FilePath};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

#[derive(Deserialize)]
struct Config {
    ip: String,
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let config = fs::read_to_string("Airq.toml").expect("Airq.toml config file not found");
    let config: Config = toml::from_str(&config).expect("Error parsing Airq.toml config file");
    let airq = AirQ::new(&config.ip, &config.password.to_string());

    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(env!("DATABASE_URL")).await?;

    let last_dbfile = sqlx::query!(
            r#"
                SELECT files.year, files.month, files.day, files.timestamp
                FROM
                    files,
                    measurements,
                    (
                        SELECT max(timestamp) as max FROM measurements
                    ) as max_measurement
                WHERE measurements.timestamp = max_measurement.max
                AND measurements.file = files.id;
            "#,
        ).fetch_optional(&pg_pool).await?
        .map(|file| FilePath { year: file.year as u16, month: file.month as u8, day: file.day as u8, timestamp: file.timestamp as u64 });
    let last_dbtimestamp = sqlx::query!("SELECT max(timestamp) as timestamp FROM measurements;")
        .fetch_optional(&pg_pool).await?
        .map(|record| record.timestamp.unwrap() as u64);

    let mut files = airq.dirbuff().await.unwrap();
    files.retain(|file| Some(file) >= last_dbfile.as_ref());

    let mut data = Vec::new();
    for file in files {
        println!("{}", file.path());
        for entry in airq.file_recrypt_data_14(&file.path()).await.unwrap() {
            data.push((file.clone(), entry));
        }
    }
    data.sort_by_key(|(_, entry)| entry.data11.timestamp);

    for (file, entry) in data {
        let Data14 {
            data11: Data11 { deviceid: _, status: _, uptime: _, health, performance, measuretime: _, timestamp, bat: _,
            door_event: _, window_open: _, tvoc, humidity, humidity_abs, humidity_abs_delta: _, temperature, dewpt, sound,
            pressure, no2, co, co2, co2_delta: _, pm1, pm2_5, pm10, cnt0_3: _, cnt0_5: _, cnt1: _, cnt2_5: _, cnt5: _,
            cnt10: _, typ_ps: _, rest: _ } , oxygen, o3, so2
        } = entry;

        if Some(timestamp) <= last_dbtimestamp {
            continue;
        }

        sqlx::query!(
                r#"
                    WITH new_file AS (
                        INSERT INTO files (year, month, day, timestamp)
                        VALUES ($1, $2, $3, $4)
                        ON CONFLICT DO NOTHING
                        RETURNING *
                    ), file AS (
                        SELECT * FROM new_file
                        UNION
                        SELECT * FROM files WHERE year = $1 AND month = $2 AND day = $3 AND timestamp = $4
                    )
                    INSERT INTO measurements VALUES (
                        $5, (SELECT id FROM file), $6, $7, $8, $9, $10, $11, $12, $13,
                        $14, $15, $16, $17, $18, $19, $20, $21, $22, $23
                    )
                    ON CONFLICT DO NOTHING
                    ;
                "#,
                file.year as i16, file.month as i16, file.day as i16, file.timestamp as i64,
                timestamp as i64, health, performance, tvoc[0], humidity[0],
                humidity_abs[0], temperature[0], dewpt[0], sound[0], pressure[0],
                no2.map(|no2| no2[0]), co.map(|co| co[0]), co2[0], pm1[0], pm2_5[0],
                pm10[0], oxygen[0], o3.map(|o3| o3[0]), so2.map(|so2| so2[0]),
            ).execute(&pg_pool).await?;
    }

    Ok(())
}