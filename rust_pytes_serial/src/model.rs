use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerData {
    pub power: u32,
    pub voltage: f32,
    pub current: f32,
    pub temperature: f32,
    pub soc: u32,
    pub basic_st: String,
    pub volt_st: Option<String>,
    pub current_st: Option<String>,
    pub temp_st: Option<String>,
    pub soh_st: Option<String>,
    pub coul_st: Option<String>,
    pub heater_st: Option<String>,
    pub bat_events: Option<u64>,
    pub power_events: Option<u64>,
    pub sys_events: Option<u64>,
    pub cycle_times: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PwrsysData {
    pub banks_total: u32,
    pub banks_current: u32,
    pub banks_sleep: u32,
    pub system_voltage: f32,
    pub system_current: f32,
    pub system_rc: f32,
    pub system_fcc: f32,
    pub system_soc: u32,
    pub system_soh: u32,
    pub system_highest_voltage: f32,
    pub system_average_voltage: f32,
    pub system_lowest_voltage: f32,
    pub system_highest_temp: f32,
    pub system_average_temp: f32,
    pub system_lowest_temp: f32,
    pub system_recommend_chg_volt: f32,
    pub system_recommend_dsg_volt: f32,
    pub system_recommend_chg_curr: f32,
    pub system_recommend_dsg_curr: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellData { pub power: u32, pub cell: u32, pub voltage: f32, pub temperature: Option<f32>, pub current: Option<f32>, pub basic_st: Option<String>, pub volt_st: Option<String>, pub curr_st: Option<String>, pub temp_st: Option<String>, pub soc: Option<u32>, pub coulomb: Option<f32> }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellPack { pub power: u32, pub voltage_delta: Option<f32>, pub voltage_min: Option<f32>, pub voltage_max: Option<f32>, pub temperature_delta: Option<f32>, pub temperature_min: Option<f32>, pub temperature_max: Option<f32>, pub cells: Vec<CellData> }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Aggregate {
    pub relay_local_time: DateTime<Utc>,
    pub powers: Option<u32>,
    pub powers_total: Option<u32>,
    pub powers_sleep: Option<u32>,
    pub voltage: Option<f32>,
    pub current: Option<f32>,
    pub system_rc: Option<f32>,
    pub system_fcc: Option<f32>,
    pub temperature: Option<f32>,
    pub soc: Option<u32>,
    pub soh: Option<u32>,
    pub highest_voltage: Option<f32>,
    pub average_voltage: Option<f32>,
    pub lowest_voltage: Option<f32>,
    pub highest_temp: Option<f32>,
    pub average_temp: Option<f32>,
    pub lowest_temp: Option<f32>,
    pub recommend_chg_volt: Option<f32>,
    pub recommend_dsg_volt: Option<f32>,
    pub recommend_chg_curr: Option<f32>,
    pub recommend_dsg_curr: Option<f32>,
    pub basic_st: Option<String>,
}

impl Aggregate {
    pub fn build(pwr: &[PowerData], pwrsys: Option<PwrsysData>, _cells: &[CellPack]) -> Self {
        let agg_voltage = if !pwr.is_empty() { Some(pwr.iter().map(|p| p.voltage).sum::<f32>() / pwr.len() as f32) } else { None };
        let agg_current = if !pwr.is_empty() { Some(pwr.iter().map(|p| p.current).sum()) } else { None };
        let agg_soc     = if !pwr.is_empty() { Some((pwr.iter().map(|p| p.soc as u64).sum::<u64>() / pwr.len() as u64) as u32) } else { None };
        let agg_temp    = if !pwr.is_empty() { Some(pwr.iter().map(|p| p.temperature).sum::<f32>() / pwr.len() as f32) } else { None };
        let basic = pwr.get(0).map(|p| p.basic_st.clone());
        if let Some(sys) = pwrsys.clone() {
            Self { relay_local_time: chrono::Utc::now(), powers: Some(sys.banks_current), powers_total: Some(sys.banks_total), powers_sleep: Some(sys.banks_sleep), voltage: Some(sys.system_voltage), current: Some(sys.system_current), system_rc: Some(sys.system_rc), system_fcc: Some(sys.system_fcc), temperature: Some(sys.system_average_temp), soc: Some(sys.system_soc), soh: Some(sys.system_soh), highest_voltage: Some(sys.system_highest_voltage), average_voltage: Some(sys.system_average_voltage), lowest_voltage: Some(sys.system_lowest_voltage), highest_temp: Some(sys.system_highest_temp), average_temp: Some(sys.system_average_temp), lowest_temp: Some(sys.system_lowest_temp), recommend_chg_volt: Some(sys.system_recommend_chg_volt), recommend_dsg_volt: Some(sys.system_recommend_dsg_volt), recommend_chg_curr: Some(sys.system_recommend_chg_curr), recommend_dsg_curr: Some(sys.system_recommend_dsg_curr), basic_st: basic }
        } else {
            Self { relay_local_time: chrono::Utc::now(), powers: Some(pwr.len() as u32), powers_total: None, powers_sleep: None, voltage: agg_voltage, current: agg_current, system_rc: None, system_fcc: None, temperature: agg_temp, soc: agg_soc, soh: None, highest_voltage: None, average_voltage: None, lowest_voltage: None, highest_temp: None, average_temp: None, lowest_temp: None, recommend_chg_volt: None, recommend_dsg_volt: None, recommend_chg_curr: None, recommend_dsg_curr: None, basic_st: basic }
        }
    }
}

#[derive(Debug, Default)]
pub struct RuntimeState { pub loops: u64, pub timestamp: DateTime<Utc>, pub last_round_trip: Duration }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitoringLevel { None, Low, Medium, High }
impl MonitoringLevel { pub fn from_str(s: &str) -> Self { match s.to_ascii_lowercase().as_str() { "low" => MonitoringLevel::Low, "medium" => MonitoringLevel::Medium, "high" => MonitoringLevel::High, _ => MonitoringLevel::None } } }
