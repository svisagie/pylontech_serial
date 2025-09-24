use crate::config_loader::AppConfig;
use anyhow::{Result, anyhow};
use serialport::{SerialPort, DataBits, Parity, StopBits, FlowControl};
use std::time::{Duration, Instant};
use log::{info};
use std::io::{Read, Write};

pub struct SerialManager { port: Box<dyn SerialPort> }

impl SerialManager {
    pub fn open(cfg: &AppConfig) -> Result<Self> {
        let port = serialport::new(&cfg.serial.serial_port, cfg.serial.serial_baudrate)
            .timeout(Duration::from_secs(10))
            .data_bits(DataBits::Eight).parity(Parity::None).stop_bits(StopBits::One).flow_control(FlowControl::None)
            .open()?;
        info!("Opened serial port {}", cfg.serial.serial_port);
        Ok(Self { port })
    }

    pub fn check_console(&mut self) -> Result<bool> {
        self.flush()?;
        let resp = self.send_and_collect("help", None::<&str>, None::<&str>, Duration::from_secs(2))?;
        if let Some(lines) = resp { Ok(lines.iter().any(|ln| ln.to_ascii_lowercase().contains("pylon"))) } else { Ok(false) }
    }

    pub fn initialize_console(&mut self) -> Result<()> {
        // reopen at 1200, send handshake, switch to 115200
        // Simplified: attempt known sequence
        self.write_raw(b"\r\n")?;
        std::thread::sleep(Duration::from_millis(500));
        self.write_raw(b"~20014682C0048520FCC3\r")?;
        std::thread::sleep(Duration::from_millis(500));
        self.write_raw(b"\r\n")?;
        self.write_line("login debug")?;
        Ok(())
    }

    fn write_raw(&mut self, data: &[u8]) -> Result<()> { self.port.write_all(data)?; self.port.flush()?; Ok(()) }
    fn write_line(&mut self, line: &str) -> Result<()> { self.write_raw(line.as_bytes())?; self.write_raw(b"\n") }
    fn flush(&mut self) -> Result<()> { self.port.clear(serialport::ClearBuffer::All)?; Ok(()) }

    pub fn send_and_collect(&mut self, cmd: &str, start_marker: Option<&str>, stop_marker: Option<&str>, timeout: Duration) -> Result<Option<Vec<String>>> {
        self.flush()?;
        self.write_line(cmd)?;
        let start_time = Instant::now();
        let mut buf = [0u8; 1024];
        let mut acc = Vec::<u8>::new();
        let mut lines: Vec<String> = Vec::new();
        let mut started = start_marker.is_none();

        while start_time.elapsed() < timeout {
            match self.port.read(&mut buf) { Ok(n) if n>0 => { acc.extend_from_slice(&buf[..n]); while let Some(pos) = acc.iter().position(|b| *b == b'\n') { let mut line = acc.drain(..=pos).collect::<Vec<u8>>(); if line.ends_with(&[b'\n']) { line.pop(); } if line.ends_with(&[b'\r']) { line.pop(); } let line_str = String::from_utf8_lossy(&line).to_string(); if !started { if let Some(sm) = start_marker { if line_str.contains(sm) { started = true; } } } if started { if let Some(stop) = stop_marker { if line_str.contains(stop) { lines.push(line_str); return Ok(Some(lines)); } } lines.push(line_str); } } }, Ok(_) => {}, Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}, Err(e) => return Err(anyhow!(e)) }
        }
        if lines.is_empty() { Ok(None) } else { Ok(Some(lines)) }
    }
}
