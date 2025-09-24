use anyhow::{Result, anyhow};
use regex::Regex;
// removed unused debug import
use crate::model::{PowerData, PwrsysData, CellPack, CellData, MonitoringLevel};

pub fn parse_pwr(power: u32, lines: &[String]) -> Result<PowerData> {
    let mut data = PowerData { power, ..Default::default() };
    for raw in lines {
        let l = raw.trim_start();
        let val = l.splitn(2,':').nth(1).unwrap_or("").trim();
        if l.starts_with("Voltage") { data.voltage = parse_scaled(val, 1000.0)?; }
        else if l.starts_with("Current Status") { data.current_st = Some(val.split_whitespace().next().unwrap_or("").to_string()); }
        else if l.starts_with("Current ") || l.starts_with("Current\t") || l == "Current" || l.starts_with("Current:") { data.current = parse_scaled(val, 1000.0)?; }
        else if l.starts_with("Temperature") { data.temperature = parse_scaled(val, 1000.0)?; }
        else if l.starts_with("Coulomb") { data.soc = parse_int(val)?; }
        else if l.starts_with("Basic Status") { data.basic_st = val.split_whitespace().next().unwrap_or("").to_string(); }
        else if l.starts_with("Volt Status") { data.volt_st = Some(val.split_whitespace().next().unwrap_or("").to_string()); }
        else if l.starts_with("Tmpr. Status") { data.temp_st = Some(val.split_whitespace().next().unwrap_or("").to_string()); }
        else if l.starts_with("Coul. Status") { data.coul_st = Some(val.split_whitespace().next().unwrap_or("").to_string()); }
        else if l.starts_with("Soh. Status") { data.soh_st = Some(val.split_whitespace().next().unwrap_or("").to_string()); }
        else if l.starts_with("Heater Status") { data.heater_st = Some(val.split_whitespace().next().unwrap_or("").to_string()); }
        else if l.starts_with("Bat Events") { data.bat_events = Some(parse_hex(val)?); }
        else if l.starts_with("Power Events") { data.power_events = Some(parse_hex(val)?); }
        else if l.starts_with("System Fault") { data.sys_events = Some(parse_hex(val)?); }
    }
    Ok(data)
}

pub fn parse_pwrsys(lines: &[String]) -> Result<PwrsysData> {
    let mut d = PwrsysData::default();
    for raw in lines {
        let l = raw.trim_start();
        let val = l.splitn(2,':').nth(1).unwrap_or("").trim();
        if l.starts_with("Total Num") { d.banks_total = parse_int(val)?; }
        else if l.starts_with("Present Num") { d.banks_current = parse_int(val)?; }
        else if l.starts_with("Sleep Num") { d.banks_sleep = parse_int(val)?; }
        else if l.starts_with("System Volt") { d.system_voltage = parse_scaled(val,1000.0)?; }
        else if l.starts_with("System Curr") { d.system_current = parse_scaled(val,1000.0)?; }
        else if l.starts_with("System RC") { d.system_rc = parse_scaled(val,1000.0)?; }
        else if l.starts_with("System FCC") { d.system_fcc = parse_scaled(val,1000.0)?; }
        else if l.starts_with("System SOC") { d.system_soc = parse_int(val)?; }
        else if l.starts_with("System SOH") { d.system_soh = parse_int(val)?; }
        else if l.starts_with("Highest voltage") { d.system_highest_voltage = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Average voltage") { d.system_average_voltage = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Lowest voltage") { d.system_lowest_voltage = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Highest temperature") { d.system_highest_temp = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Average temperature") { d.system_average_temp = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Lowest temperature") { d.system_lowest_temp = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Recommend chg voltage") { d.system_recommend_chg_volt = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Recommend dsg voltage") { d.system_recommend_dsg_volt = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Recommend chg current") { d.system_recommend_chg_curr = parse_scaled(val,1000.0)?; }
        else if l.starts_with("Recommend dsg current") { d.system_recommend_dsg_curr = parse_scaled(val,1000.0)?; }
    }
    Ok(d)
}

pub fn parse_stat(power: u32, lines: &[String], pwr_list: &mut Vec<PowerData>) -> Result<()> {
    // find entry
    if let Some(entry) = pwr_list.iter_mut().find(|p| p.power == power) {
        for l in lines { if l.contains("CYCLE Times     :") { entry.cycle_times = Some(slice_int(l,18,27)? as u64); } }
    }
    Ok(())
}

pub fn parse_bat(power: u32, lines: &[String], _expected_cells: u32, level: &MonitoringLevel) -> Result<CellPack> {
    let mut cells: Vec<CellData> = Vec::new();
    if lines.len() < 2 { return Err(anyhow!("not enough lines")); }
    // header
    let header = &lines[0];
    let _parts: Vec<&str> = header.split_whitespace().collect();
    // dynamic columns not fully matched; we'll regex each data line with >=2 spaces delim
    let re = Regex::new(r"\s{2,}").unwrap();
    for (i, l) in lines.iter().enumerate() { if i==0 || l.contains("Command completed") { continue; } let cols: Vec<&str> = re.split(l.trim()).collect(); if cols.len()<2 { continue; } let mut cell = CellData::default(); cell.power = power; if let Ok(cn) = cols.get(0).unwrap_or(&"0").parse::<u32>() { cell.cell = cn + 1; } if let Some(vs) = cols.get(1) { if let Ok(v) = vs.parse::<i32>() { cell.voltage = v as f32 / 1000.0; } } if matches!(level, MonitoringLevel::Medium | MonitoringLevel::High) { if let Some(ts) = cols.get(3) { if let Ok(t) = ts.parse::<i32>() { cell.temperature = Some(t as f32 / 1000.0); } } } if matches!(level, MonitoringLevel::High) { // attempt mapping further indexes heuristically
            cell.current = cols.get(2).and_then(|s| s.parse::<i32>().ok()).map(|v| v as f32 / 1000.0);
        }
        cells.push(cell); }
    let mut pack = CellPack { power, cells: cells.clone(), ..Default::default() };
    if matches!(level, MonitoringLevel::Medium | MonitoringLevel::High) {
        if !cells.is_empty() { let min_v = cells.iter().map(|c| c.voltage).fold(f32::INFINITY, f32::min); let max_v = cells.iter().map(|c| c.voltage).fold(f32::NEG_INFINITY, f32::max); pack.voltage_min = Some(min_v); pack.voltage_max = Some(max_v); pack.voltage_delta = Some((max_v - min_v).abs()); }
    }
    Ok(pack)
}

fn slice_str(s: &str, a: usize, b: usize) -> String { s.chars().skip(a).take(b-a).collect::<String>().trim().to_string() }
fn parse_int(token: &str) -> Result<u32> { let core = token.split_whitespace().find(|t| t.chars().any(|c| c.is_ascii_digit())).unwrap_or("0").trim_matches('%'); Ok(core.parse().unwrap_or(0)) }
fn parse_hex(token: &str) -> Result<u64> { let core = token.split_whitespace().find(|t| t.contains('x') || t.chars().any(|c| c.is_ascii_hexdigit())).unwrap_or("0x0").trim_start_matches("0x"); Ok(u64::from_str_radix(core,16).unwrap_or(0)) }
fn parse_scaled(token: &str, div: f32) -> Result<f32> { let core = token.split_whitespace().find(|t| t.chars().any(|c| c.is_ascii_digit())).unwrap_or("0"); let digits: String = core.chars().take_while(|c| c.is_ascii_digit()).collect(); if digits.is_empty() { return Ok(0.0); } Ok(digits.parse::<i32>().unwrap_or(0) as f32 / div) }

fn extract_number_segment(s: &str) -> Option<&str> {
    // take portion after ':' then split by whitespace keeping first token containing digit/hex
    let after_colon = s.splitn(2, ':').nth(1)?.trim();
    after_colon.split_whitespace().find(|tok| tok.chars().any(|c| c.is_ascii_digit()))
}

fn slice_int(s: &str, a: usize, b: usize) -> Result<u32> {
    // fallback to dynamic extraction if fixed slice empty
    let seg = slice_str(s,a,b);
    if seg.is_empty() { if let Some(tok) = extract_number_segment(s) { return Ok(tok.trim_matches('%').parse()?); } }
    Ok(seg.trim_matches('%').parse()?)
}

// removed legacy fixed-slice numeric helpers (slice_hex, take_num) no longer used after dynamic parsing refactor
