use airq::{AirQ, Data11, Data14, FilePath};
use thiserror::Error;
use futures::{future, TryFutureExt, stream::{self, StreamExt, TryStreamExt}};
use sqlx::{Connection, PgPool};
use chrono::{Duration, Utc};

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("airq error: {0}")]
    Airq(#[from] airq::Error),
}

pub struct FetchData {
    airq: AirQ,
}

impl FetchData {
    pub fn new(ip: &str, password: &str) -> FetchData {
        let airq = AirQ::new(ip, password);
        FetchData { airq }
    }

    pub async fn fetch_current(&self) -> Result<Data14, Error> {
        Ok(self.airq.data_14().await?)
    }

    pub async fn fetch_data(&self, last_file: Option<FilePath>, last_timestamp: Option<u64>) -> Result<(FilePath, Vec<Data14>), Error> {
        // self.airq.test().await;
        dbg!(self.airq.log().await)?;
        let dirs = self.airq.dirbuff().await?;
        let from = match last_timestamp {
            Some(ts) => ts,
            None => (Utc::now() - Duration::days(7)).timestamp() as u64,
        };
        let mut timestamps: Vec<_> = dirs.into_iter().filter(|f| f.timestamp >= from).collect();

        const CONCURRENT_REQUESTS: usize = 3;
        timestamps.sort_unstable();
        println!("{} timestamps", timestamps.len());

        let mut entries = stream::iter(timestamps)
            .inspect(|file| println!("fetching {}", file.path()))
            .map(|file| self.airq.file_data_14(file.path()).map_ok(move |data| (file, data)).map_err(Error::from))
            .buffer_unordered(CONCURRENT_REQUESTS);

        let mut data = Vec::new();
        let mut last_file = FilePath {
            year: 0,
            month: 0,
            day: 0,
            timestamp: 0
        };
        while let Some((file, entries)) = entries.next().await.transpose()? {
            data.extend(entries);
            last_file = last_file.max(file);
        }
        Ok((last_file, data))
    }

    // pub async fn fetch_data(&self, pg_pool: &PgPool) -> Result<(), Error> {
    //     let last_dbfile = sqlx::query!(
    //         r#"
    //             SELECT files.year, files.month, files.day, files.timestamp as file_timestamp, measurements.timestamp as measurement_timestamp
    //             FROM files, measurements
    //             WHERE measurements.file = files.id
    //             ORDER BY measurements.timestamp DESC
    //             LIMIT 1;
    //         "#,
    //     ).fetch_optional(pg_pool).await?;
    //
    //     let last_dbtimestamp = last_dbfile.as_ref().map(|record| record.measurement_timestamp as u64);
    //     let last_dbfile = last_dbfile.map(|record| FilePath {
    //         year: record.year as u16,
    //         month: record.month as u8,
    //         day: record.day as u8,
    //         timestamp: record.file_timestamp as u64,
    //     });
    //     let last_dbfile_year = last_dbfile.map(|p| p.year);
    //     let last_dbfile_month = last_dbfile.map(|p| p.month);
    //     let last_dbfile_day = last_dbfile.map(|p| p.day);
    //     let last_dbfile_timestamp = last_dbfile.map(|p| p.timestamp);
    //
    //     const CONCURRENT_REQUESTS: usize = 3;
    //     // fetch directories
    //     let years = self.airq.dir("/").await?.into_iter()
    //         .filter_map(|year| year.parse().ok())
    //         .filter(|&year| Some(year) >= last_dbfile_year)
    //         .collect::<Vec<_>>();
    //     println!("{} years", years.len());
    //     let months = stream::iter(years)
    //         .map(|year| self.airq.dir(format!("/{}", year)).map_ok(move |months| (year, months)))
    //         .buffer_unordered(CONCURRENT_REQUESTS)
    //         .map_ok(|(year, months)| stream::iter(months).map(move |month| -> Result<_, Error> { Ok((year, month.parse::<u8>().unwrap())) }))
    //         .try_flatten()
    //         .try_filter(|&(year, month)| future::ready(Some(year) > last_dbfile_year || Some(month) >= last_dbfile_month))
    //         .try_collect::<Vec<_>>().await?;
    //     println!("{} months", months.len());
    //     let days = stream::iter(months)
    //         .map(|(year, month)| self.airq.dir(format!("/{}/{}", year, month)).map_ok(move |days| (year, month, days)))
    //         .buffer_unordered(CONCURRENT_REQUESTS)
    //         .map_ok(|(year, month, days)| stream::iter(days).map(move |day| -> Result<_, Error> { Ok((year, month, day.parse().unwrap())) }))
    //         .try_flatten()
    //         .try_filter(|&(year, month, day)| future::ready(Some(year) > last_dbfile_year || Some(month) > last_dbfile_month || Some(day) >= last_dbfile_day))
    //         .try_collect::<Vec<_>>().await?;
    //     println!("{} days", days.len());
    //     let mut timestamps = stream::iter(days)
    //         .map(|(year, month, day)| self.airq.dir(format!("/{}/{}/{}", year, month, day)).map_ok(move |timestamps| (year, month, day, timestamps)))
    //         .buffer_unordered(CONCURRENT_REQUESTS)
    //         .map_ok(|(year, month, day, timestamps)| stream::iter(timestamps).map(move |timestamp| -> Result<_, Error> { Ok((year, month, day, timestamp.parse().unwrap())) }))
    //         .try_flatten()
    //         .try_filter(|&(year, month, day, timestamp)| future::ready(Some(year) > last_dbfile_year || Some(month) > last_dbfile_month || Some(day) > last_dbfile_day || Some(timestamp) >= last_dbfile_timestamp))
    //         .map_ok(|(year, month, day, timestamp)| FilePath { year, month, day, timestamp })
    //         .try_collect::<Vec<_>>().await?;
    //     timestamps.sort_unstable();
    //     println!("{} timestamps", timestamps.len());
    //
    //     let mut entries = stream::iter(timestamps)
    //         .inspect(|file| println!("fetching {}", file.path()))
    //         .map(|file| self.airq.file_recrypt_data_14(file.path()).map_ok(move |entries| (file, entries)).map_err(Error::from))
    //         // .buffer_unordered(CONCURRENT_REQUESTS);
    //         .buffer_unordered(1);
    //
    //     while let Some((file, entries)) = entries.next().await.transpose()? {
    //         pg_pool.acquire().await?.transaction::<_, _, Error>(move |conn| Box::pin(async move {
    //             for entry in entries {
    //                 let Data14 {
    //                     data11: Data11 {
    //                         deviceid: _, status: _, uptime: _, health, performance, measuretime: _, timestamp, bat: _,
    //                         door_event: _, window_open: _, tvoc, humidity, humidity_abs, humidity_abs_delta: _, temperature, dewpt, sound,
    //                         pressure, no2, co, co2, co2_delta: _, pm1, pm2_5, pm10, cnt0_3: _, cnt0_5: _, cnt1: _, cnt2_5: _, cnt5: _,
    //                         cnt10: _, typ_ps: _, rest: _
    //                     }, oxygen, o3, so2
    //                 } = entry;
    //                 if Some(entry.data11.timestamp) <= last_dbtimestamp {
    //                     continue;
    //                 }
    //                 sqlx::query!(
    //                     r#"
    //                         WITH new_file AS (
    //                             INSERT INTO files (year, month, day, timestamp)
    //                             VALUES ($1, $2, $3, $4)
    //                             ON CONFLICT DO NOTHING
    //                             RETURNING *
    //                         ), file AS (
    //                             SELECT * FROM new_file
    //                             UNION
    //                             SELECT * FROM files WHERE year = $1 AND month = $2 AND day = $3 AND timestamp = $4
    //                         )
    //                         INSERT INTO measurements VALUES (
    //                             $5, (SELECT id FROM file), $6, $7, $8, $9, $10, $11, $12, $13,
    //                             $14, $15, $16, $17, $18, $19, $20, $21, $22, $23
    //                         )
    //                         ON CONFLICT DO NOTHING
    //                         ;
    //                     "#,
    //                     file.year as i16, file.month as i16, file.day as i16, file.timestamp as i64,
    //                     timestamp as i64, health, performance, tvoc[0], humidity[0],
    //                     humidity_abs[0], temperature[0], dewpt[0], sound[0], pressure[0],
    //                     no2.map(|no2| no2[0]), co.map(|co| co[0]), co2[0], pm1[0], pm2_5[0],
    //                     pm10[0], oxygen[0], o3.map(|o3| o3[0]), so2.map(|so2| so2[0]),
    //                 ).execute(&mut *conn).await?;
    //             }
    //             Ok(())
    //         })).await?;
    //     }
    //     Ok(())
    // }
}

