use std::fs::{self, File};
use std::io::Write;
use std::collections::{HashMap, HashSet};

use airq::{AirQ, Data11, Data14};
use serde::Deserialize;
use clap::Clap;

#[derive(Deserialize)]
struct Config {
    ip: String,
    password: String,
}

#[derive(Clap)]
enum Args {
    Server,
    FetchData,
}

fn main() {
    let config = fs::read_to_string("Airq.toml").expect("Airq.toml config file not found");
    let config: Config = toml::from_str(&config).expect("Error parsing Airq.toml config file");
    let airq = AirQ::new(&config.ip, &config.password.to_string());
    // println!("{:?}", airq.blink());
    // println!("{:?}", airq.config());
    // println!("{:?}", airq.ping());
    // println!("{:?}", airq.standardpass());
    // println!("{:?}", airq.dir("2020/8/7"));
    // println!("{:?}", airq.log());

    // panic!();

    let mut data: Vec<_> = airq.dirbuff().unwrap().into_iter()
        .inspect(|file| println!("{}", file.path()))
        .flat_map(|file| airq.file_recrypt_data_14(&file.path()).unwrap())
        .collect();
    data.sort_by_key(|entry| entry.data11.timestamp);

    let mut timestamps = HashSet::new();
    let mut map: HashMap<_, Vec<_>> = HashMap::new();
    for entry in data {
        let Data14 {
            data11: Data11 { deviceid: _, status: _, uptime: _, health, performance, measuretime: _, timestamp, bat: _,
            door_event: _, window_open: _, tvoc, humidity, humidity_abs, humidity_abs_delta: _, temperature, dewpt, sound,
            pressure, no2, co, co2, co2_delta: _, pm1, pm2_5, pm10, cnt0_3: _, cnt0_5: _, cnt1: _, cnt2_5: _, cnt5: _,
            cnt10: _, typ_ps: _, rest: _ } , oxygen, o3, so2
        } = entry;
        if timestamps.contains(&timestamp) {
            println!("duplicate timestamp {}", timestamp);
            continue;
        }
        timestamps.insert(timestamp);
        map.entry("timestamp").or_default().push(timestamp as f64);
        map.entry("health").or_default().push(health);
        map.entry("performance").or_default().push(performance);
        map.entry("tvoc").or_default().push(tvoc[0]);
        map.entry("humidity").or_default().push(humidity[0]);
        map.entry("humidity_abs").or_default().push(humidity_abs[0]);
        map.entry("temperature").or_default().push(temperature[0]);
        map.entry("dewpt").or_default().push(dewpt[0]);
        map.entry("sound").or_default().push(sound[0]);
        map.entry("pressure").or_default().push(pressure[0]);
        map.entry("no2").or_default().push(no2.map(|no2| no2[0]).unwrap_or_default());
        map.entry("co").or_default().push(co.map(|co| co[0]).unwrap_or_default());
        map.entry("co2").or_default().push(co2[0]);
        map.entry("pm1").or_default().push(pm1[0]);
        map.entry("pm2_5").or_default().push(pm2_5[0]);
        map.entry("pm10").or_default().push(pm10[0]);
        map.entry("oxygen").or_default().push(oxygen[0]);
        map.entry("o3").or_default().push(o3.map(|o3| o3[0]).unwrap_or_default());
        map.entry("so2").or_default().push(so2.map(|so2| so2[0]).unwrap_or_default());
    }

    let mut file = File::create("data.js").unwrap();
    write!(file, "data = ").unwrap();
    serde_json::to_writer(file, &map).unwrap();
}