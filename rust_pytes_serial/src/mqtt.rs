use anyhow::{Result, anyhow};
use rumqttc::{MqttOptions, Client, QoS, LastWill, Event, Packet};
use serde_json::json;
use crate::config_loader::AppConfig;
use crate::model::{Aggregate, PowerData, CellPack, MonitoringLevel};
use crate::events::EventsSummary;
// log debug not used currently
use std::time::Duration;
use std::collections::HashMap;

#[derive(Default)]
struct Cache { values: HashMap<String, String> }

pub struct MqttClient { 
    pub client: Client, 
    pub dev_name: String, 
    cache: Cache, 
    ha_prefix: String,
    pub rediscovery_done: bool,
}

impl MqttClient {
    pub fn connect(cfg: &AppConfig) -> Result<(Self, tokio::sync::mpsc::Receiver<()>)> {
    let mut opts = MqttOptions::new("rust_pytes_serial", &cfg.mqtt.mqtt_broker, cfg.mqtt.mqtt_port);
        opts.set_keep_alive(Duration::from_secs(60));
    opts.set_credentials(&cfg.mqtt.mqtt_username, &cfg.mqtt.mqtt_password);
        // assume non-TLS broker unless port 8883 typical
    #[cfg(feature = "tls")]
    if cfg.mqtt.mqtt_port == 8883 { use rumqttc::Transport; opts.set_transport(Transport::tls_with_default_config()); }
        let avail = format!("pytes_serial/{}/availability", cfg.battery_info.dev_name);
        opts.set_last_will(LastWill::new(&avail, "offline", QoS::AtLeastOnce, true));
        let (client, mut connection) = Client::new(opts, 30);
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        std::thread::spawn(move || {
            for ev in connection.iter() {
                if let Ok(Event::Incoming(Packet::ConnAck(_))) = ev { let _ = tx.blocking_send(()); }
            }
        });
        Ok((Self { client, dev_name: cfg.battery_info.dev_name.clone(), cache: Cache::default(), ha_prefix: "homeassistant".into(), rediscovery_done: false }, rx))
    }

    pub fn publish_availability(&mut self, state: &str) -> Result<()> { self.client.publish(format!("pytes_serial/{}/availability", self.dev_name), QoS::AtLeastOnce, true, state).map_err(|e| anyhow!(e)) }

    pub fn discovery(&mut self, cfg: &AppConfig) -> Result<()> {
        let dev = json!({"identifiers":[self.dev_name],"manufacturer":cfg.battery_info.manufacturer,"model":cfg.battery_info.model,"name":self.dev_name,"sw_version":"rust_pytes_serial 0.1.0"});
        let sys_defs = [
            ("current","Current","A","current"),
            ("voltage","Voltage","V","voltage"),
            ("soc","SOC","%","battery"),
            ("temperature","Temperature","°C","temperature"),
            ("system_rc","Remaining Capacity","AH","battery"),
            ("system_fcc","Full Charge Capacity","AH","battery"),
            ("soh","System SOH","%","battery"),
            ("highest_voltage","Highest Cell Voltage","V","voltage"),
            ("lowest_voltage","Lowest Cell Voltage","V","voltage"),
            ("average_voltage","Average Cell Voltage","V","voltage"),
            ("highest_temp","Highest Cell Temp","°C","temperature"),
            ("lowest_temp","Lowest Cell Temp","°C","temperature"),
            ("average_temp","Average Cell Temp","°C","temperature"),
        ];
    for (id,name,unit,dev_cla) in sys_defs { self.publish_discovery_sensor(&dev, id, name, unit, Some(dev_cla), None, &format!("pytes_serial/{}/{}", self.dev_name, id))?; }

        // per power
        for p in 0..cfg.battery_info.powers { for (id,name,unit,dev_cla) in [
            ("voltage","Voltage","V","voltage"),
            ("current","Current","A","current"),
            ("temperature","Temperature","°C","temperature"),
            ("soc","SOC","%","battery"),
        ] { let uniq = format!("{}_{}_{}", self.dev_name, id, p+1); let topic = format!("{}/sensor/{}/{}_config/config", self.ha_prefix, self.dev_name, uniq); let payload = json!({"uniq_id":uniq, "name": format!("{}_{}", name, p+1), "stat_t": format!("pytes_serial/{}/{}/{}", self.dev_name, p, id), "dev_cla": dev_cla, "unit_of_meas": unit, "val_tpl": "{{ value_json.value }}", "dev": dev }); self.client.publish(topic, QoS::AtLeastOnce, true, payload.to_string())?; } }

        // cells discovery (subset) depending on level
        let level = MonitoringLevel::from_str(&cfg.cells_monitoring.monitoring_level);
        if cfg.cells_monitoring.cells_monitoring && level != MonitoringLevel::None { let cell_metrics = match level { MonitoringLevel::Low => vec![ ("voltage","Voltage","V","voltage") ], MonitoringLevel::Medium => vec![ ("voltage","Voltage","V","voltage"), ("temperature","Temperature","°C","temperature") ], MonitoringLevel::High => vec![ ("voltage","Voltage","V","voltage"), ("temperature","Temperature","°C","temperature"), ("current","Current","A","current") ], MonitoringLevel::None => vec![] }; for p in 0..cfg.battery_info.powers { for c in 0..cfg.battery_info.cells { for (id,name,unit,dev_cla) in &cell_metrics { let uniq = format!("{}_{}_{}_{}", self.dev_name, id, p+1, c+1); let topic = format!("{}/sensor/{}/{}_config/config", self.ha_prefix, self.dev_name, uniq); let payload = json!({"uniq_id":uniq, "name": format!("{}{}_{}{:02}", name, if name.ends_with('e'){""} else {""}, p+1, c+1), "stat_t": format!("pytes_serial/{}/{}/cells/{}/{}", self.dev_name, p, c, id), "dev_cla": dev_cla, "unit_of_meas": unit, "val_tpl": "{{ value_json.value }}", "dev": dev }); self.client.publish(topic, QoS::AtLeastOnce, true, payload.to_string())?; } } } }
        Ok(())
    }

    pub fn publish_runtime(&mut self, _cfg: &AppConfig, agg: &Aggregate, pwr: &[PowerData], cells: &[CellPack], events: &EventsSummary) -> Result<()> {
        // helper with caching
        let mut publish = |topic: String, value: serde_json::Value, retain: bool| -> Result<()> {
            let payload = value.to_string();
            if self.cache.values.get(&topic).map(|v| v == &payload).unwrap_or(false) { return Ok(()); }
            self.client.publish(topic.clone(), QoS::AtLeastOnce, retain, payload.clone()).map_err(|e| anyhow!(e))?;
            self.cache.values.insert(topic, payload);
            Ok(())
        };

        macro_rules! pubopt { ($k:expr,$v:expr) => { if let Some(val) = $v { publish(format!("pytes_serial/{}/{}", self.dev_name,$k), json!({"value":val}), false)?; } }; }
        pubopt!("current", agg.current); pubopt!("voltage", agg.voltage); pubopt!("soc", agg.soc); pubopt!("temperature", agg.temperature);

    for pd in pwr { let idx = pd.power -1; publish(format!("pytes_serial/{}/{}/voltage", self.dev_name, idx), json!({"value": pd.voltage}), false)?; publish(format!("pytes_serial/{}/{}/current", self.dev_name, idx), json!({"value": pd.current}), false)?; }

        // cells subset
    for pack in cells { let pidx = pack.power -1; for c in &pack.cells { let cidx = c.cell -1; publish(format!("pytes_serial/{}/{}/cells/{}/voltage", self.dev_name, pidx, cidx), json!({"value": c.voltage}), false)?; if let Some(t) = c.temperature { publish(format!("pytes_serial/{}/{}/cells/{}/temperature", self.dev_name, pidx, cidx), json!({"value": t}), false)?; } } }

        // events counters
        publish(format!("pytes_serial/{}/events", self.dev_name), json!({"bat_events_no": events.bat_events_no, "pwr_events_no": events.pwr_events_no, "sys_events_no": events.sys_events_no }), false)?;
        Ok(())
    }

    fn publish_discovery_sensor(&mut self, dev: &serde_json::Value, id: &str, name: &str, unit: &str, dev_cla: Option<&str>, stat_cla: Option<&str>, stat_topic: &str) -> Result<()> {
        let uniq = format!("{}_{}", self.dev_name, id);
        let topic = format!("{}/sensor/{}/{}_config/config", self.ha_prefix, self.dev_name, uniq);
        let mut payload = json!({
            "uniq_id": uniq,
            "name": name,
            "stat_t": stat_topic,
            "unit_of_meas": unit,
            "val_tpl": "{{ value_json.value }}",
            "dev": dev
        });
        if let Some(c) = dev_cla { payload["dev_cla"] = json!(c); }
        if let Some(c) = stat_cla { payload["stat_cla"] = json!(c); }
        self.client.publish(topic, QoS::AtLeastOnce, true, payload.to_string()).map_err(|e| anyhow!(e))
    }
}
