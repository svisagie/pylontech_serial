mod mqtt;
mod serial_layer;
mod parsing;
mod discovery;
mod model;
mod config_loader;
mod events;

use crate::config_loader::AppConfig;
use crate::serial_layer::{SerialManager};
use crate::mqtt::MqttClient;
use crate::parsing::{parse_pwr, parse_pwrsys, parse_stat, parse_bat};
use crate::model::*;
use crate::events::*;

use anyhow::Result;
use clap::Parser;
use std::time::{Duration, Instant};
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc::Receiver;
use chrono::Utc;
use log::{info, warn, error};
use tokio::signal;

#[derive(Parser, Debug)]
#[command(author, version, about="Pytes serial to MQTT bridge (Rust)")]
struct Cli { #[arg(long, help="Run without serial device and generate synthetic data")] mock: bool }

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = match AppConfig::load("pytes_serial.cfg") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}\nPlease ensure 'pytes_serial.cfg' exists in the current directory or a parent directory.");
            std::process::exit(2);
        }
    };
    // logging level optional from config
    let level = cfg.logging.logging_level.clone().unwrap_or_else(||"info".into());
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).format_timestamp_secs().init();
    info!("START - rust_pytes_serial v0.1.0");
    info!("Loaded config: serial_port={} baud={} powers={} cells={}", cfg.serial.serial_port, cfg.serial.serial_baudrate, cfg.battery_info.powers, cfg.battery_info.cells);

    // Serial (skipped in mock)
    let mut serial = if cli.mock { None } else {
        match SerialManager::open(&cfg) {
            Ok(mut s) => { if !s.check_console()? { info!("Console not responding, attempting initialization sequence"); s.initialize_console()?; if !s.check_console()? { error!("Initialization failed"); return Ok(()); } } Some(s) },
            Err(e) => { error!("Serial open failed: {e:#}. Use --mock to run without hardware."); return Ok(()); }
        }
    };

    // MQTT
    let (mqtt_client_arc, mut reconnect_rx): (Option<Arc<AsyncMutex<MqttClient>>>, Option<Receiver<()>>) = if cfg.mqtt.mqtt_active { 
        let (cli, rx) = MqttClient::connect(&cfg)?; 
        let arc = Arc::new(AsyncMutex::new(cli));
        { let mut lock = arc.lock().await; lock.publish_availability("online").ok(); lock.discovery(&cfg).ok(); }
        (Some(arc), Some(rx))
    } else { (None, None) };

    // Reconnect handler
    if let (Some(arc), Some(mut rx)) = (mqtt_client_arc.clone(), reconnect_rx.take()) {
        let cfg_clone = cfg.clone();
        tokio::spawn(async move {
            while let Some(_) = rx.recv().await {
                let mut c = arc.lock().await;
                if let Err(e) = c.publish_availability("online") { warn!("reconnect availability publish failed: {e:#}"); }
                if !c.rediscovery_done { if let Err(e) = c.discovery(&cfg_clone) { warn!("rediscovery failed: {e:#}"); } c.rediscovery_done = true; }
            }
        });
    }

    // Shared state
    let state = Arc::new(Mutex::new(RuntimeState::default()));

    let reading_freq = Duration::from_secs(cfg.serial.reading_freq.max(5) as u64);
    let mut last_stat = Instant::now() - Duration::from_secs(cfg.stat_parsing.parsing_stat_interval as u64 + 10);

    // Graceful shutdown task
    if let Some(arc) = mqtt_client_arc.clone() {
        tokio::spawn(async move {
            #[cfg(unix)] let _ = signal::unix::signal(signal::unix::SignalKind::terminate());
            // ctrl_c covers most platforms
            let _ = signal::ctrl_c().await;
            let mut c = arc.lock().await; let _ = c.publish_availability("offline");
            std::process::exit(0);
        });
    }

    loop {
        let loop_start = Instant::now();
        {
            let mut st = state.lock();
            st.loops += 1;
            st.timestamp = Utc::now();
        }

        // Parse power blocks
        let mut all_pwr: Vec<PowerData> = Vec::new();
        if cli.mock {
            for p in 1..=cfg.battery_info.powers { all_pwr.push(PowerData { power: p, voltage: 51.2 + p as f32 * 0.05, current: 5.0 + p as f32, temperature: 24.0 + p as f32, soc: 80, basic_st: "OK".into(), ..Default::default() }); }
        } else if let Some(ser) = serial.as_mut() {
            for p in 1..=cfg.battery_info.powers {
                match ser.send_and_collect(&format!("pwr {}", p), Some(&format!("pwr {}", p)), Some("Command completed"), Duration::from_secs(5))? {
                    Some(lines) => { match parse_pwr(p, &lines) { Ok(data) => all_pwr.push(data), Err(e) => { warn!("parse_pwr error: {e:?}"); } } },
                    None => warn!("No response for pwr {p}")
                }
            }
        }

        // pwrsys
    let pwrsys = if cli.mock { Some(PwrsysData { banks_total: cfg.battery_info.powers, banks_current: cfg.battery_info.powers, banks_sleep: 0, system_voltage: 51.6, system_current: 8.2, system_rc: 90.0, system_fcc: 100.0, system_soc: 90, system_soh: 98, system_highest_voltage: 3.25*16.0, system_average_voltage: 3.22*16.0, system_lowest_voltage: 3.20*16.0, system_highest_temp: 26.0, system_average_temp: 25.0, system_lowest_temp: 24.0, system_recommend_chg_volt: 54.0, system_recommend_dsg_volt: 44.0, system_recommend_chg_curr: 50.0, system_recommend_dsg_curr: 50.0 }) } else if let Some(ser) = serial.as_mut() { ser.send_and_collect("pwrsys", Some("Power System Information"), Some("Command completed"), Duration::from_secs(5))? .and_then(|lines| parse_pwrsys(&lines).ok()) } else { None };

        // stat
        if last_stat.elapsed().as_secs() as i64 >= cfg.stat_parsing.parsing_stat_interval {
            if cli.mock { for p in 1..=cfg.battery_info.powers { if let Some(e) = all_pwr.iter_mut().find(|pp| pp.power==p) { e.cycle_times = Some(120); } } }
            else if let Some(ser) = serial.as_mut() { for p in 1..=cfg.battery_info.powers { if let Some(lines) = ser.send_and_collect(&format!("stat {}", p), Some("Device address"), Some("Command completed"), Duration::from_secs(5))? { let _ = parse_stat(p, &lines, &mut all_pwr); } } }
            last_stat = Instant::now();
        }

        // cells
        let mut cells_data: Vec<CellPack> = Vec::new();
        let mon_level = MonitoringLevel::from_str(&cfg.cells_monitoring.monitoring_level);
        if cfg.cells_monitoring.cells_monitoring && mon_level != MonitoringLevel::None {
            if cli.mock {
                for p in 1..=cfg.battery_info.powers { let mut pack = CellPack { power: p, ..Default::default() }; for c in 1..=cfg.battery_info.cells { pack.cells.push(CellData { power: p, cell: c, voltage: 3.20 + (c as f32 * 0.002), temperature: Some(25.0 + (c as f32 * 0.05)), ..Default::default() }); } cells_data.push(pack); }
            } else if let Some(ser) = serial.as_mut() {
                for p in 1..=cfg.battery_info.powers { if let Some(lines) = ser.send_and_collect(&format!("bat {}", p), Some("Battery"), Some("Command completed"), Duration::from_secs(6))? { if let Ok(c) = parse_bat(p, &lines, cfg.battery_info.cells, &mon_level) { cells_data.push(c); } } }
            }
        }

        // Build aggregate
        let aggregate = Aggregate::build(&all_pwr, pwrsys, &cells_data);

        // Events detection (counts only for now)
        let events_summary = detect_events(&all_pwr, &aggregate);

        // Publish MQTT (with basic error logging)
        if let Some(arc) = &mqtt_client_arc { 
            let mut client = arc.lock().await; 
            if let Err(e) = client.publish_runtime(&cfg, &aggregate, &all_pwr, &cells_data, &events_summary) { warn!("mqtt publish error: {e:#}"); }
        }

        {
            let mut st = state.lock();
            st.last_round_trip = loop_start.elapsed();
        }

        // sleep until next loop
        let elapsed = loop_start.elapsed();
        if reading_freq > elapsed { tokio::time::sleep(reading_freq - elapsed).await; } else { tokio::task::yield_now().await; }
    }
}
