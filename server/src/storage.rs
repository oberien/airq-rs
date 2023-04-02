use std::fs::File;
use std::sync::Mutex;
use sqlx::{Connection, PgPool};
use sqlx::postgres::PgPoolOptions;
use async_trait::async_trait;
use chrono::{Datelike, Duration, Utc};
use futures::{Stream, StreamExt};
use airq::{Data11, Data14, FilePath};
use serde::{Serialize, Deserialize};
use crate::{Timestamps, Measurement};
use crate::fetch_data::Error;

#[async_trait]
pub trait MeasurementStorage: Send + Sync {
    // async fn last_file(&self) -> Option<FilePath>;
    // async fn last_timestamp(&self) -> u64;
    async fn timestamps(&self) -> Result<Timestamps, Error>;
    async fn data(&self, first: u64, last: u64, combine_datapoints: u64, combine_millis: u64, ) -> Result<Vec<Measurement>, Error>;
    async fn last_timestamps(&self) -> Result<Option<(FilePath, u64)>, Error>;
    async fn store_entries(&self, entries: &mut (dyn Stream<Item = (FilePath, Vec<Data14>)> + Unpin + Send), last_timestamp: Option<u64>) -> Result<(), Error>;
    async fn store_manual_readout(&self, data: &Data14) -> Result<(), Error>;
    async fn clean_manual_readouts(&self) -> Result<(), Error>;
}

#[derive(Default, Serialize, Deserialize)]
pub struct Sevendays {
    last_file: Mutex<Option<FilePath>>,
    data: Mutex<Vec<Data14>>,
}

pub struct Postgres {
    pool: PgPool,
}

impl Sevendays {
    pub fn open() -> Sevendays {
        let file = match File::open("sevendays.json") {
            Ok(file) => file,
            Err(_) => return Sevendays::default(),
        };
        serde_json::from_reader(file).unwrap()
    }
}
#[async_trait]
impl MeasurementStorage for Sevendays {
    async fn timestamps(&self) -> Result<Timestamps, Error> {
        let data = self.data.lock().unwrap();
        let first = data.first().map(|data| data.data11.timestamp as i64);
        let last = data.last().map(|data| data.data11.timestamp as i64);
        Ok(Timestamps { first, last })
    }

    async fn data(&self, first: u64, last: u64, combine_datapoints: u64, _combine_millis: u64) -> Result<Vec<Measurement>, Error> {
        Ok(self.data.lock().unwrap().chunks(combine_datapoints as usize)
            .map(|data| {
                fn avg(data: &[Data14], f: impl Fn(&Data14) -> f64) -> f64 {
                    data.iter().map(f).sum::<f64>() / data.len() as f64
                }
                Measurement {
                    timestamp: Some(data[0].data11.timestamp as i64),
                    health: Some(avg(data, |entry| entry.data11.health)),
                    performance: Some(avg(data, |entry| entry.data11.performance)),
                    tvoc: Some(avg(data, |entry| entry.data11.tvoc.unwrap_or_default()[0])),
                    humidity: Some(avg(data, |entry| entry.data11.humidity[0])),
                    humidity_abs: Some(avg(data, |entry| entry.data11.humidity_abs[0])),
                    temperature: Some(avg(data, |entry| entry.data11.temperature[0])),
                    dewpt: Some(avg(data, |entry| entry.data11.dewpt[0])),
                    sound: Some(avg(data, |entry| entry.data11.sound[0])),
                    pressure: Some(avg(data, |entry| entry.data11.pressure[0])),
                    no2: Some(avg(data, |entry| entry.data11.no2.unwrap_or_default()[0])),
                    co: Some(avg(data, |entry| entry.data11.co.unwrap_or_default()[0])),
                    co2: Some(avg(data, |entry| entry.data11.co2[0])),
                    pm1: Some(avg(data, |entry| entry.data11.pm1[0])),
                    pm2_5: Some(avg(data, |entry| entry.data11.pm2_5[0])),
                    pm10: Some(avg(data, |entry| entry.data11.pm10[0])),
                    oxygen: Some(avg(data, |entry| entry.oxygen[0])),
                    o3: Some(avg(data, |entry| entry.o3.unwrap_or_default()[0])),
                    so2: Some(avg(data, |entry| entry.so2.unwrap_or_default()[0])),
                }
            })
            .skip_while(|data| data.timestamp.unwrap() < first as i64)
            .take_while(|data| data.timestamp.unwrap() <= last as i64)
            .collect())
    }

    async fn last_timestamps(&self) -> Result<Option<(FilePath, u64)>, Error> {
        let last_file = self.last_file.lock().unwrap().clone();
        let last_timestamp = self.data.lock().unwrap().last().map(|data| data.data11.timestamp);
        let seven_days_ago = Utc::now() - Duration::days(7);
        let from_file = last_file.unwrap_or(FilePath {
            year: seven_days_ago.year() as u16,
            month: seven_days_ago.month() as u8,
            day: seven_days_ago.day() as u8,
            timestamp: seven_days_ago.timestamp() as u64,
        });
        let from_ts = match last_timestamp {
            Some(ts) => ts,
            None => seven_days_ago.timestamp() as u64,
        };
        Ok(Some((from_file, from_ts)))
    }

    async fn store_entries(&self, entries: &mut (dyn Stream<Item = (FilePath, Vec<Data14>)> + Unpin + Send), last_timestamp: Option<u64>) -> Result<(), Error> {
        // collect
        let mut collected_data = Vec::new();
        let mut collected_last_file = FilePath {
            year: 0,
            month: 0,
            day: 0,
            timestamp: 0
        };
        while let Some((file, entries)) = entries.next().await {
            collected_data.extend(entries.into_iter().filter(|data| data.data11.timestamp > last_timestamp.unwrap()));
            collected_last_file = collected_last_file.max(file);
        }

        // append
        let mut data = self.data.lock().unwrap();
        let mut last_file = self.last_file.lock().unwrap();
        let last_ts = data.last()
            .map(|data| data.data11.timestamp)
            .unwrap_or_default();
        let to_add = collected_data.into_iter().skip_while(|data| data.data11.timestamp <= last_ts);
        data.extend(to_add);
        *last_file = Some(collected_last_file);
        data.sort_unstable_by_key(|data| data.data11.timestamp);
        serde_json::to_writer(File::create("sevendays.json").unwrap(), &*data).unwrap();

        Ok(())
    }

    // not needed for local testing
    async fn store_manual_readout(&self, _data: &Data14) -> Result<(), Error> { Ok(()) }
    async fn clean_manual_readouts(&self) -> Result<(), Error> { Ok(()) }
}

impl Postgres {
    pub async fn connect() -> Postgres {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(&std::env::var("DATABASE_URL").unwrap()).await
            .unwrap();
        Postgres { pool }
    }
}
#[async_trait]
impl MeasurementStorage for Postgres {
    async fn timestamps(&self) -> Result<Timestamps, Error> {
        Ok(sqlx::query_as!(
            Timestamps,
            "SELECT extract(epoch from min(timestamp))::int8 * 1000 as first, extract(epoch from max(timestamp))::int8 * 1000 as last FROM measurements;"
        ).fetch_one(&self.pool).await?)
    }

    async fn data(&self, first: u64, last: u64, _combine_datapoints: u64, combine_millis: u64) -> Result<Vec<Measurement>, Error> {
        Ok(sqlx::query_as!(
                Measurement,
                r#"
                    SELECT
                        extract(epoch from min(timestamp))::int8 * 1000 as timestamp, avg(health) as health, avg(performance) as performance,
                        avg(tvoc) as tvoc, avg(humidity) as humidity, avg(humidity_abs) as humidity_abs,
                        avg(temperature) as temperature, avg(dewpt) as dewpt, avg(sound) as sound,
                        avg(pressure) as pressure, avg(no2) as no2, avg(co) as co,
                        avg(co2) as co2, avg(pm1) as pm1, avg(pm2_5) as pm2_5,
                        avg(pm10) as pm10, avg(oxygen) as oxygen, avg(o3) as o3,
                        avg(so2) as so2
                    FROM measurements
                    WHERE extract(epoch from timestamp)::int8 * 1000 >= $1 AND extract(epoch from timestamp)::int8 * 1000 <= $2
                    GROUP BY extract(epoch from timestamp)::int8 * 1000 / $3
                    ORDER BY timestamp;
                "#,
                first as i64, last as i64, combine_millis as i64
            ).fetch_all(&self.pool).await?)
    }

    async fn last_timestamps(&self) -> Result<Option<(FilePath, u64)>, Error> {
        let last = sqlx::query!(
            r#"
                SELECT files.year, files.month, files.day, files.timestamp as file_timestamp, extract(epoch from measurements.timestamp)::int8 * 1000 as measurement_timestamp
                FROM files, measurements
                WHERE measurements.file = files.id
                ORDER BY measurements.timestamp DESC
                LIMIT 1;
            "#,
        ).fetch_optional(&self.pool).await?;
        Ok(last.map(|last| (FilePath {
            year: last.year as u16,
            month: last.month as u8,
            day: last.day as u8,
            timestamp: last.file_timestamp as u64,
        }, last.measurement_timestamp.unwrap() as u64)))
    }

    async fn store_entries(&self, entries: &mut (dyn Stream<Item = (FilePath, Vec<Data14>)> + Unpin + Send), last_timestamp: Option<u64>) -> Result<(), Error> {
        while let Some((file, entries)) = entries.next().await {
            self.pool.acquire().await?.transaction::<_, _, Error>(move |conn| Box::pin(async move {
                for entry in entries {
                    let Data14 {
                        data11: Data11 {
                            deviceid: _, status: _, uptime: _, health, performance, measuretime: _, timestamp, bat: _,
                            door_event: _, window_open: _, tvoc, humidity, humidity_abs, humidity_abs_delta: _, temperature, dewpt, sound,
                            pressure, no2, co, co2, co2_delta: _, pm1, pm2_5, pm10, cnt0_3: _, cnt0_5: _, cnt1: _, cnt2_5: _, cnt5: _,
                            cnt10: _, typ_ps: _, rest: _
                        }, oxygen, o3, so2
                    } = entry;
                    if Some(entry.data11.timestamp) <= last_timestamp {
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
                                to_timestamp($5 / 1000), (SELECT id FROM file), $6, $7, $8, $9, $10, $11, $12, $13,
                                $14, $15, $16, $17, $18, $19, $20, $21, $22, $23
                            )
                            ON CONFLICT DO NOTHING
                            ;
                        "#,
                        file.year as i16, file.month as i16, file.day as i16, file.timestamp as i64,
                        timestamp as i64, health, performance, tvoc.map(|tvoc| tvoc[0]), humidity[0],
                        humidity_abs[0], temperature[0], dewpt[0], sound[0], pressure[0],
                        no2.map(|no2| no2[0]), co.map(|co| co[0]), co2[0], pm1[0], pm2_5[0],
                        pm10[0], oxygen[0], o3.map(|o3| o3[0]), so2.map(|so2| so2[0]),
                    ).execute(&mut *conn).await?;
                }
                Ok(())
            })).await?;
        }
        Ok(())
    }

    async fn store_manual_readout(&self, data: &Data14) -> Result<(), Error> {
        let Data14 {
            data11: Data11 {
                deviceid: _, status: _, uptime: _, health, performance, measuretime: _, timestamp, bat: _,
                door_event: _, window_open: _, tvoc, humidity, humidity_abs, humidity_abs_delta: _, temperature, dewpt, sound,
                pressure, no2, co, co2, co2_delta: _, pm1, pm2_5, pm10, cnt0_3: _, cnt0_5: _, cnt1: _, cnt2_5: _, cnt5: _,
                cnt10: _, typ_ps: _, rest: _
            }, oxygen, o3, so2
        } = data;
        sqlx::query!(
            r#"
                INSERT INTO measurements VALUES (
                    to_timestamp($1 / 1000), NULL, $2, $3, $4, $5, $6, $7, $8, $9,
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19
                )
                ON CONFLICT DO NOTHING
                ;
            "#,
            *timestamp as i64, health, performance, tvoc.map(|tvoc| tvoc[0]), humidity[0],
            humidity_abs[0], temperature[0], dewpt[0], sound[0], pressure[0],
            no2.map(|no2| no2[0]), co.map(|co| co[0]), co2[0], pm1[0], pm2_5[0],
            pm10[0], oxygen[0], o3.map(|o3| o3[0]), so2.map(|so2| so2[0]),
        ).execute(&self.pool).await?;
        Ok(())
    }

    async fn clean_manual_readouts(&self) -> Result<(), Error> {
        sqlx::query!(r#"
            DELETE FROM measurements
            WHERE file IS NULL AND timestamp < (NOW() - INTERVAL '7 DAYS');
        "#).execute(&self.pool).await?;
        Ok(())
    }
}
