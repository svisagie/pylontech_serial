use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SerialCfg { pub serial_port: String, pub serial_baudrate: u32, pub reading_freq: i64 }
#[derive(Debug, Clone)]
pub struct BatteryInfoCfg { pub powers: u32, pub cells: u32, pub dev_name: String, pub manufacturer: String, pub model: String }
#[derive(Debug, Clone)]
pub struct MqttCfg { pub mqtt_active: bool, pub mqtt_broker: String, pub mqtt_port: u16, pub mqtt_username: String, pub mqtt_password: String }
#[derive(Debug, Clone)]
pub struct CellsMonitoringCfg { pub cells_monitoring: bool, pub monitoring_level: String }
#[derive(Debug, Clone)]
pub struct StatParsingCfg { pub parsing_stat_interval: i64 }
#[derive(Debug, Clone, Default)]
pub struct LoggingCfg { pub logging_level: Option<String> }

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub serial: SerialCfg,
    pub battery_info: BatteryInfoCfg,
    pub mqtt: MqttCfg,
    pub cells_monitoring: CellsMonitoringCfg,
    pub stat_parsing: StatParsingCfg,
    pub logging: LoggingCfg,
}

impl AppConfig {
    pub fn load_env() -> Result<Self> {
        let env = |k: &str| std::env::var(k).ok();
        let parse = |k: &str, default: &str| env(k).unwrap_or_else(|| default.to_string());
        let boolp = |k: &str, default: bool| env(k).map(|v| v.eq_ignore_ascii_case("true") || v=="1").unwrap_or(default);

        // Mapping (ENV_VAR -> former config key):
        // SERIAL_PORT -> serial.serial_port
        // SERIAL_BAUDRATE -> serial.serial_baudrate
        // READING_FREQ -> serial.reading_freq
        // POWERS -> battery_info.powers
        // CELLS -> battery_info.cells
        // DEV_NAME -> battery_info.dev_name
        // MANUFACTURER -> battery_info.manufacturer
        // MODEL -> battery_info.model
        // MQTT_ACTIVE -> MQTT.MQTT_active
        // MQTT_BROKER -> MQTT.MQTT_broker
        // MQTT_PORT -> MQTT.MQTT_port
        // MQTT_USERNAME -> MQTT.MQTT_username
        // MQTT_PASSWORD -> MQTT.MQTT_password
        // CELLS_MONITORING -> cells_monitoring.cells_monitoring
        // MONITORING_LEVEL -> cells_monitoring.monitoring_level
        // PARSING_STAT_INTERVAL -> stat_parsing.parsing_stat_interval
        // LOGGING_LEVEL -> logging.LOGGING_LEVEL

        let serial = SerialCfg {
            serial_port: parse("SERIAL_PORT", "COM3"),
            serial_baudrate: parse("SERIAL_BAUDRATE", "115200").parse().unwrap_or(115200),
            reading_freq: parse("READING_FREQ", "10").parse().unwrap_or(10),
        };
        let battery_info = BatteryInfoCfg {
            powers: parse("POWERS", "1").parse().unwrap_or(1),
            cells: parse("CELLS", "16").parse().unwrap_or(16),
            dev_name: parse("DEV_NAME", "pytes"),
            manufacturer: parse("MANUFACTURER", "PYTES Energy Co.Ltd"),
            model: parse("MODEL", "E-BOX-48100R"),
        };
        let mqtt = MqttCfg {
            mqtt_active: boolp("MQTT_ACTIVE", false),
            mqtt_broker: parse("MQTT_BROKER", "127.0.0.1"),
            mqtt_port: parse("MQTT_PORT", "1883").parse().unwrap_or(1883),
            mqtt_username: parse("MQTT_USERNAME", ""),
            mqtt_password: parse("MQTT_PASSWORD", ""),
        };
        let mut monitoring_level = parse("MONITORING_LEVEL", "none").to_lowercase();
        let valid_levels = ["none","medium","high"]; 
        if !valid_levels.contains(&monitoring_level.as_str()) { 
            eprintln!("[config] Invalid MONITORING_LEVEL='{}' -> falling back to 'none'", monitoring_level); 
            monitoring_level = "none".into();
        }
        let cells_monitoring = CellsMonitoringCfg { cells_monitoring: boolp("CELLS_MONITORING", false), monitoring_level };
        let stat_parsing = StatParsingCfg { parsing_stat_interval: parse("PARSING_STAT_INTERVAL", "60").parse().unwrap_or(60) };
        let logging = LoggingCfg { logging_level: env("LOGGING_LEVEL") };

        Ok(AppConfig { serial, battery_info, mqtt, cells_monitoring, stat_parsing, logging })
    }
}
