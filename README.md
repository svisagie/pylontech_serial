### PYTES E-BOX 48100R / PYLONTECH to Home Assistant / MariaDB 
Program is reading RS232 serial port of PYTES and PYLONTECH LiFePo4 batteries.

### How does this software work?
"pwr", "bat", "pwrsys" and "stat" commands are used.
Program reads serial port with a specific frequency, parsing the data and saving a JSON file that can be used in further automation. 

Configurable OPTIONS:
- record data to MariaDB
- send data via MQTT
- events monitoring - when power_events, battery_events or system faults occurred a log file is created with cells details 
- cell monitoring - read cells details for all batteries in bank. Three levels of details can be selected in config file.
  more details/explanation can be found [here](/docs/configuration_details.txt)
  
These options can be activated / deactivated in configuration file (pytes_serial.cfg)

When MQTT is activated:
- JSON file is send as payload to the following topic: 'homeassistant/sensor/pytes/state'
- program has build-in integration with Home Assisant where the following sensors will be automatic created for each battery:"current", "voltage", "temperature", "soc", "status".
   The battery number is embedded at the end of each sensor (i.e current_1, current_2...).

   When cell monitoring is activated an additional device will be created in Home Assistant with suffix "_cells" with all associated sensors. The battery and cell number is embedded at the end of each sensor ( i.e. voltage_102 means voltage for battery 1 cell 2).
   Basic statistics is implemented too. Therefore, additional sensors will be available for min, max and delta for cells voltage and temperature. 
   (i.e. pytes_cells_voltage_max_1 means max cells voltage for battery 1)  
  
  If more sensors will be needed, they can be added manually as per Home Assistant documentation [MQTT sensor](https://www.home-assistant.io/integrations/sensor.mqtt/) and the example in docs folder [here](/docs/home_assistant_add_sensor.txt).

You have more [examples](/examples) for better understanding of what program does.

Thanks to [chinezbrun's pytes_serial](https://github.com/chinezbrun/pytes_serial) from which this was forked and got to where it is now -- possibly a bit more Pylontech centric

### Installation and Execution
Serial cable must be connected to battery 1 (master).
1. copy current repository 
2. optional:
   a. if you want to use MariaDB:
      - MariaDB database must be installed (MariaDB documentation is out of this project scope)
      - use sql/pytes_mariadb.sql to import required database and tables
      
   b. if you want to use MQTT / MQTT integration in Home Assistant:
    - MQTT broker must be installed (MQTT documentation is out of this project scope)
    - if you use Home Assistant, make sure that MQTT auto discovery is set true and sensors will be auto discovered when program will start
3. rename pytes_serial.cfg.example in pytes_serial.cfg and configure it as per your needs (do not remark or delete lines in sections just do the configuration)
4. make sure that all required python modules are installed see [requirements](/REQUIREMENTS.md)
5. go to the folder where the program is located (i.e cd /home/pi/Documents/pytes)
6. execute pytes_serial.sh to have a separate terminal instance (works for Linux/Raspberry) or python3 pytes_serial.py directly from console.
   if you need setup an autostart of the program on reboot more info [here](/docs/) 

A lighter version written in Micropython for ESP32 is available here:[pytes_esp](https://github.com/chinezbrun/pytes_esp)

enjoy

---

## Rust Implementation (Environment-Only Configuration)

The Rust port (`rust_pytes_serial`) no longer uses `pytes_serial.cfg`. All configuration is supplied via environment variables (with sane defaults). This allows containerized deployments without mounting a config file.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SERIAL_PORT` | `COM3` | Serial device path (e.g. `/dev/ttyUSB0` on Linux). |
| `SERIAL_BAUDRATE` | `115200` | Serial baud rate. |
| `READING_FREQ` | `10` | Main loop frequency in seconds (minimum enforced 5). |
| `POWERS` | `1` | Number of battery packs (power segments) to poll. |
| `CELLS` | `16` | Number of cells per pack for cell monitoring logic. |
| `DEV_NAME` | `pytes` | Device base name used in MQTT discovery. |
| `MANUFACTURER` | `PYTES Energy Co.Ltd` | Manufacturer metadata. |
| `MODEL` | `E-BOX-48100R` | Model metadata. |
| `MQTT_ACTIVE` | `false` | Enable MQTT publishing (`true` / `false` / `1`). |
| `MQTT_BROKER` | `127.0.0.1` | MQTT broker host/IP. |
| `MQTT_PORT` | `1883` | MQTT broker port. |
| `MQTT_USERNAME` | (empty) | MQTT username (optional). |
| `MQTT_PASSWORD` | (empty) | MQTT password (optional). |
| `CELLS_MONITORING` | `false` | Enable cell polling. |
| `MONITORING_LEVEL` | `none` | One of `none`, `medium`, `high` (invalid falls back to `none`). |
| `PARSING_STAT_INTERVAL` | `60` | Seconds between `stat` command polls. |
| `LOGGING_LEVEL` | `info` | Log level if `RUST_LOG` not set. |
| `RUST_LOG` | (unset) | Overrides logging filter (standard env_logger syntax). |

### Example (Docker)
```bash
docker run --rm \
   --device=/dev/ttyUSB0 \
   -e SERIAL_PORT=/dev/ttyUSB0 \
   -e POWERS=1 -e CELLS=16 \
   -e MQTT_ACTIVE=true -e MQTT_BROKER=192.168.1.10 \
   -e LOGGING_LEVEL=debug \
   ghcr.io/svisagie/pylon_serial:rust-latest
```

### Mock Mode
Run with the `--mock` CLI flag to generate synthetic data without a serial device:
```bash
./rust_pytes_serial --mock
```

### Migration Notes
If you previously used `pytes_serial.cfg`, translate each key to the variable listed above. No file mount is required; remove any `PYTES_CFG` references in your deployment manifests.

### Multi-Architecture Images
The GitHub Actions workflow builds and publishes a multi-arch image for `linux/amd64`, `linux/arm64` (64‑bit Raspberry Pi), and `linux/arm/v7` (32‑bit Raspberry Pi). Pulling `ghcr.io/svisagie/pylon_serial:rust-latest` on a Pi will automatically select the correct variant.

