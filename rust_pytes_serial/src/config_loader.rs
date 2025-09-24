use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct SerialCfg { pub serial_port: String, pub serial_baudrate: u32, pub reading_freq: i64 }
#[derive(Debug, Deserialize, Clone)]
pub struct BatteryInfoCfg { pub powers: u32, pub cells: u32, pub dev_name: String, pub manufacturer: String, pub model: String }
#[derive(Debug, Deserialize, Clone)]
pub struct MqttCfg { 
    pub mqtt_active: bool, 
    pub mqtt_broker: String, 
    pub mqtt_port: u16, 
    pub mqtt_username: String, 
    pub mqtt_password: String 
}
#[derive(Debug, Deserialize, Clone)]
pub struct CellsMonitoringCfg { pub cells_monitoring: bool, pub monitoring_level: String }
#[derive(Debug, Deserialize, Clone)]
pub struct StatParsingCfg { pub parsing_stat_interval: i64 }

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LoggingCfg { pub logging_level: Option<String> }

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub serial: SerialCfg,
    pub battery_info: BatteryInfoCfg,
    pub mqtt: MqttCfg,
    pub cells_monitoring: CellsMonitoringCfg,
    pub stat_parsing: StatParsingCfg,
    pub logging: LoggingCfg,
}

impl AppConfig {
    pub fn load(path: &str) -> Result<Self> {
        // Allow environment variable override
        let env_override = std::env::var("PYTES_CFG").ok();
        let mut search_list: Vec<std::path::PathBuf> = Vec::new();
        if let Some(p) = env_override { search_list.push(p.into()); }
        search_list.push(std::path::PathBuf::from(path));
        search_list.push(std::path::PathBuf::from("pytes_serial.cfg"));
        if let Ok(cwd) = std::env::current_dir() { search_list.push(cwd.join("pytes_serial.cfg")); }
        if let Ok(cwd) = std::env::current_dir() { if let Some(p) = cwd.parent() { search_list.push(p.join("pytes_serial.cfg")); } }
        if let Ok(cwd) = std::env::current_dir() { if let Some(p) = cwd.parent().and_then(|p| p.parent()) { search_list.push(p.join("pytes_serial.cfg")); } }
        // Example fallback
        search_list.push(std::path::PathBuf::from("pytes_serial.cfg.example"));

        let mut content_opt = None;
        for cand in &search_list { if cand.exists() { content_opt = Some(fs::read_to_string(cand)?); break; } }
        let content = content_opt.ok_or_else(|| anyhow!("Configuration file 'pytes_serial.cfg' (or example) not found. Tried: {:?}", search_list))?;
        // Simple INI style manual parse (limited) because existing file is INI
        let mut current = String::new();
        use std::collections::HashMap;
        let mut map: HashMap<String, HashMap<String,String>> = HashMap::new();
        for line in content.lines() { let line = line.trim(); if line.is_empty() || line.starts_with('#') || line.starts_with(';') { continue; } if line.starts_with('[') && line.ends_with(']') { current = line[1..line.len()-1].to_string(); continue; } if let Some((k,v)) = line.split_once('=') { map.entry(current.clone()).or_default().insert(k.trim().to_string(), v.trim().to_string()); } }
        let g = |sec: &str, key: &str| -> Result<String> { map.get(sec).and_then(|m| m.get(key)).cloned().ok_or_else(|| anyhow!("Missing {sec}.{key}")) };

        let serial = SerialCfg { 
            serial_port: g("serial","serial_port")?, 
            serial_baudrate: g("serial","serial_baudrate")?.parse()?, 
            reading_freq: g("serial","reading_freq").unwrap_or_else(|_|"10".into()).parse()? 
        };
        let battery_info = BatteryInfoCfg { powers: g("battery_info","powers")?.parse()?, cells: g("battery_info","cells")?.parse()?, dev_name: g("battery_info","dev_name")?, manufacturer: g("battery_info","manufacturer")?, model: g("battery_info","model")? };
    let mqtt = MqttCfg { mqtt_active: g("MQTT","MQTT_active").unwrap_or_else(|_|"false".into()).eq_ignore_ascii_case("true"), mqtt_broker: g("MQTT","MQTT_broker")?, mqtt_port: g("MQTT","MQTT_port")?.parse()?, mqtt_username: g("MQTT","MQTT_username")?, mqtt_password: g("MQTT","MQTT_password")? };
        let cells_monitoring = CellsMonitoringCfg { cells_monitoring: g("cells_monitoring","cells_monitoring").unwrap_or_else(|_|"false".into()).eq_ignore_ascii_case("true"), monitoring_level: g("cells_monitoring","monitoring_level").unwrap_or_else(|_|"none".into()) };
    let stat_parsing = StatParsingCfg { parsing_stat_interval: g("stat_parsing","parsing_stat_interval").unwrap_or_else(|_|"60".into()).parse()? };
    let logging = LoggingCfg { logging_level: map.get("logging").and_then(|m| m.get("LOGGING_LEVEL")).cloned() };

        Ok(AppConfig { serial, battery_info, mqtt, cells_monitoring, stat_parsing, logging })
    }
}
