use airq::{AirQ, Data14, FilePath};
use thiserror::Error;
use futures::{future, TryFutureExt, stream::{self, StreamExt, TryStreamExt}};
use chrono::{Duration, Utc};
use crate::MeasurementStorage;

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

    pub async fn fetch_data(&self, storage: &dyn MeasurementStorage) -> Result<(), Error> {
        let last_timestamps = storage.last_timestamps().await?;

        const CONCURRENT_REQUESTS: usize = 3;

        // dirbuff contains the current and last month
        let dirbuff_start = (Utc::now() - Duration::days(25)).timestamp() as u64;
        let mut timestamps: Vec<_> = match last_timestamps {
            // use dirbuff as an optimization if possible
            Some((from_file, from_ts)) if dirbuff_start < from_ts => {
                let files = self.airq.dirbuff().await?;
                files.into_iter().filter(|f| *f >= from_file).collect()
            },
            // fall back to full directory listing
            _ => {
                let last_file_year = last_timestamps.map(|(p, _)| p.year);
                let last_file_month = last_timestamps.map(|(p, _)| p.month);
                let last_file_day = last_timestamps.map(|(p, _)| p.day);
                let last_file_timestamps = last_timestamps.map(|(p, _)| p.timestamp);

                let years = self.airq.dir("/").await?.into_iter()
                    .filter_map(|year| year.parse().ok())
                    .filter(|&year| Some(year) >= last_file_year)
                    .collect::<Vec<_>>();
                println!("{} years", years.len());
                let months = stream::iter(years)
                    .map(|year| self.airq.dir(format!("/{}", year)).map_ok(move |months| (year, months)))
                    .buffer_unordered(CONCURRENT_REQUESTS)
                    .map_ok(|(year, months)| stream::iter(months).map(move |month| -> Result<_, Error> { Ok((year, month.parse::<u8>().unwrap())) }))
                    .try_flatten()
                    .try_filter(|&(year, month)| future::ready(Some(year) > last_file_year || Some(month) >= last_file_month))
                    .try_collect::<Vec<_>>().await?;
                println!("{} months", months.len());
                let days = stream::iter(months)
                    .map(|(year, month)| self.airq.dir(format!("/{}/{}", year, month)).map_ok(move |days| (year, month, days)))
                    .buffer_unordered(CONCURRENT_REQUESTS)
                    .map_ok(|(year, month, days)| stream::iter(days).map(move |day| -> Result<_, Error> { Ok((year, month, day.parse().unwrap())) }))
                    .try_flatten()
                    .try_filter(|&(year, month, day)| future::ready(Some(year) > last_file_year || Some(month) > last_file_month || Some(day) >= last_file_day))
                    .try_collect::<Vec<_>>().await?;
                println!("{} days", days.len());
                let timestamps = stream::iter(days)
                    .map(|(year, month, day)| self.airq.dir(format!("/{}/{}/{}", year, month, day)).map_ok(move |timestamps| (year, month, day, timestamps)))
                    .buffer_unordered(CONCURRENT_REQUESTS)
                    .map_ok(|(year, month, day, timestamps)| stream::iter(timestamps).map(move |timestamp| -> Result<_, Error> { Ok((year, month, day, timestamp.parse().unwrap())) }))
                    .try_flatten()
                    .try_filter(|&(year, month, day, timestamp)| future::ready(Some(year) > last_file_year || Some(month) > last_file_month || Some(day) > last_file_day || Some(timestamp) >= last_file_timestamps))
                    .map_ok(|(year, month, day, timestamp)| FilePath { year, month, day, timestamp })
                    .try_collect::<Vec<_>>().await?;
                timestamps
            }
        };

        timestamps.sort_unstable();
        println!("{} timestamp(s)", timestamps.len());

        let mut entries = stream::iter(timestamps)
            .inspect(|file| println!("fetching {}", file.path()))
            .map(|file| self.airq.file_data_14(file.path()).map_ok(move |data| (file, data)))
            .buffer_unordered(CONCURRENT_REQUESTS)
            .inspect_err(|e| eprintln!("Error fetching data from airQ: {:?}", e))
            .filter_map(|res| future::ready(res.ok()))
            ;

        storage.store_entries(&mut entries, last_timestamps.map(|(_, ts)| ts)).await?;
        Ok(())
    }

    // pub async fn fetch_data(&self, pg_pool: &PgPool) -> Result<(), Error> {
    // }
}

