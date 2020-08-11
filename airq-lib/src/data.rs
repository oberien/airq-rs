use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Encrypted {
    #[serde(rename = "id")]
    pub(crate) deviceid: Option<String>,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceId {
    pub id: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Deserialize)]
pub struct FilePath {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub timestamp: u64,
}

impl FilePath {
    pub fn path(&self) -> String {
        format!("{}/{}/{}/{}", self.year, self.month, self.day, self.timestamp)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ping {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Status {
    Ok(String),
    WarmUp {
        co: Option<String>,
        no2: Option<String>,
        o3: Option<String>,
        so2: Option<String>,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Data11 {
    #[serde(rename = "DeviceID")]
    pub deviceid: String,
    #[serde(rename = "Status")]
    pub status: Status,
    pub uptime: u64,
    pub health: f64,
    pub performance: f64,
    pub measuretime: f64,
    pub timestamp: u64,
    pub bat: [f64; 2],
    pub door_event: f64,
    pub window_open: Option<f64>,
    // values
    pub tvoc: [f64; 2],
    pub humidity: [f64; 2],
    pub humidity_abs: [f64; 2],
    #[serde(rename = "dHdt")]
    pub humidity_abs_delta: f64,
    pub temperature: [f64; 2],
    /// Dew Point Temperature
    pub dewpt: [f64; 2],
    pub sound: [f64; 2],
    pub pressure: [f64; 2],
    pub no2: Option<[f64; 2]>,
    pub co: Option<[f64; 2]>,
    pub co2: [f64; 2],
    #[serde(rename = "dCO2dt")]
    pub co2_delta: f64,
    pub pm1: [f64; 2],
    pub pm2_5: [f64; 2],
    pub pm10: [f64; 2],
    pub cnt0_3: [f64; 2],
    pub cnt0_5: [f64; 2],
    pub cnt1: [f64; 2],
    pub cnt2_5: [f64; 2],
    pub cnt5: [f64; 2],
    pub cnt10: [f64; 2],
    #[serde(rename = "TypPS")]
    pub typ_ps: f64,
    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Data14 {
    #[serde(flatten)]
    pub data11: Data11,
    pub oxygen: [f64; 2],
    pub o3: Option<[f64; 2]>,
    pub so2: Option<[f64; 2]>,
}

impl fmt::Display for Data11 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Data11 { deviceid: _, status: _, uptime: _, health, performance, measuretime: _, timestamp: _, bat: _,
            door_event: _, window_open: _, tvoc, humidity, humidity_abs, humidity_abs_delta: _, temperature, dewpt, sound,
            pressure, no2, co, co2, co2_delta: _, pm1, pm2_5, pm10, cnt0_3: _, cnt0_5: _, cnt1: _, cnt2_5: _, cnt5: _,
            cnt10: _, typ_ps: _, rest: _ } = self;
        writeln!(f, "health: {}", health)?;
        writeln!(f, "performance: {}", performance)?;
        writeln!(f, "tvoc: {}ppb", tvoc[0])?;
        writeln!(f, "humidity: {}%", humidity[0])?;
        writeln!(f, "humidity_abs: {}g/m³", humidity_abs[0])?;
        writeln!(f, "temperature: {}°C", temperature[0])?;
        writeln!(f, "dewpoint: {}°C", dewpt[0])?;
        writeln!(f, "sound: {}dB(A)", sound[0])?;
        writeln!(f, "pressure: {}hPa", pressure[0])?;
        if let Some(no2) = no2 {
            writeln!(f, "NO₂: {}μg/m³", no2[0])?;
        } else {
            writeln!(f, "SO₂: initializing")?;
        }
        if let Some(co) = co {
            writeln!(f, "CO: {}mg/m³", co[0])?;
        } else {
            writeln!(f, "CO: initializing")?;
        }
        writeln!(f, "CO₂: {}ppm", co2[0])?;
        writeln!(f, "PM 1: {}μg/m³", pm1[0])?;
        writeln!(f, "PM 2.5: {}μg/m³", pm2_5[0])?;
        writeln!(f, "PM 10: {}μg/m³", pm10[0])?;
        Ok(())
    }
}
impl fmt::Display for Data14 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Data14 { data11, oxygen, o3, so2 } = self;
        data11.fmt(f)?;
        writeln!(f, "O₂: {}μg/m³", oxygen[0])?;
        if let Some(o3) = o3 {
            writeln!(f, "O₃: {}μg/m³", o3[0])?;
        } else {
            writeln!(f, "O₃: initializing")?;
        }
        if let Some(so2) = so2 {
            writeln!(f, "SO₂: {}μg/m³", so2[0])?;
        } else {
            writeln!(f, "SO₂: initializing")?;
        }
        Ok(())
    }
}
