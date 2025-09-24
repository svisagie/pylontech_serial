# rust_pytes_serial

Rust reimplementation of the original `pytes_serial.py` focusing on:
- Serial polling of Pylontech/Pytes console commands (`pwr`, `pwrsys`, `stat`, `bat`)
- Aggregation of pack & cell metrics
- Streaming to MQTT (basic Home Assistant discovery – minimal subset so far)
- Skips MariaDB / file JSON persistence (can be added later)

## Status
Initial port (alpha). Parsing offsets mirror Python logic; may need adjustment for firmware variations. Cell parsing is heuristic and simplified.

## Reconnection & Availability
The MQTT client:
- Uses internal rumqttc reconnect loop.
- Publishes `availability` = `online` at startup and after each successful reconnect.
- Publishes `availability` = `offline` on graceful shutdown (Ctrl+C) or via broker LWT if the process dies unexpectedly.
- Performs Home Assistant discovery once at startup and once again on the first reconnect (cached afterward).

## Build
```bash
cd rust_pytes_serial
# Default (no TLS to avoid native build tools requirement)
cargo build --release

# Enable TLS (requires MSVC build tools for ring on Windows)
cargo build --release --features tls
```

## Run
Ensure `pytes_serial.cfg` exists at workspace root (same as Python script). Then:
```bash
cd rust_pytes_serial
RUST_LOG=info cargo run --release
```

### Mock Mode (No Hardware Required)
Generate synthetic pack and cell data without opening a serial port:
```bash
RUST_LOG=info cargo run --release -- --mock
```
This mode fabricates stable but changing-looking values so you can validate MQTT + discovery workflows.

## Config Expectations
Reads the same INI sections/keys used by the Python version:
- [serial] serial_port, serial_baudrate, reading_freq
- [battery_info] powers, cells, dev_name, manufacturer, model
- [MQTT] MQTT_active, MQTT_broker, MQTT_port, MQTT_username, MQTT_password
- [cells_monitoring] cells_monitoring (true/false), monitoring_level (low|medium|high)
- [stat_parsing] parsing_stat_interval

## Home Assistant Discovery
Publishes sensors for aggregate metrics:
`current`, `voltage`, `soc`, `temperature`, `system_rc`, `system_fcc`, `soh`, `highest_voltage`, `lowest_voltage`, `average_voltage`, `highest_temp`, `lowest_temp`, `average_temp`.

Per‑battery sensors: voltage, current, temperature, soc.

Optional per‑cell sensors (level driven):
- low: voltage
- medium: voltage, temperature
- high: voltage, temperature, current (experimental parsing)

Extend or adjust definitions in `src/mqtt.rs` (`discovery()` method).

## Testing
Unit tests cover core parsing helpers:
```bash
cargo test
```

## Future Enhancements
- Event code mapping & descriptive text
- Additional sensors (cycle counts, relay state, more status flags)
- Structured logging (JSON option)
- Fixture-based parser tests using captured console output
- Optional binary flag to force rediscovery every N minutes
- Direct JSON export (parity with original script)

## Safety & Error Handling
Errors in command parsing currently log and continue. Serial layer uses timeouts to avoid blocking the loop.

## License
Dual licensed MIT / Apache-2.0.
