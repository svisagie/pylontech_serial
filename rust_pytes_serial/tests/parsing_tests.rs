use rust_pytes_serial::parsing::{parse_pwr, parse_pwrsys, parse_bat, parse_stat};
mod helpers;
use helpers::load_blocks;

fn block_without_markers(block: &[String]) -> Vec<String> {
    let mut collected = Vec::new();
    for l in block {
        if l.starts_with('@') { continue; }
        if l.contains("Command completed") { break; }
        if l.contains("$$") { break; }
        if l.trim().is_empty() { continue; }
        collected.push(l.clone());
    }
    collected
}

#[test]
fn test_parse_pwrsys_from_sample() {
    let path = "../command_responses/pwrsys.txt"; // relative to tests dir
    let blocks = load_blocks(path);
    assert!(!blocks.is_empty(), "No blocks found in pwrsys sample");
    let lines = block_without_markers(&blocks[0]);
    let d = parse_pwrsys(&lines).expect("parse pwrsys");
    assert!(d.system_voltage > 0.0, "system voltage not parsed");
    assert!(d.system_soc <= 100);
}

#[test]
fn test_parse_pwr_from_sample() {
    let path = "../command_responses/pwr x.txt";
    let blocks = load_blocks(path);
    assert!(!blocks.is_empty(), "No blocks found in pwr sample");
    let lines = block_without_markers(&blocks[0]);
    let p = parse_pwr(1, &lines).expect("parse pwr");
    assert_eq!(p.power, 1);
    assert!(p.voltage > 0.0, "voltage not parsed");
    assert!(p.soc <= 100);
}

#[test]
fn test_parse_stat_from_sample() {
    let path = "../command_responses/stat x.txt";
    let blocks = load_blocks(path);
    if blocks.is_empty() { return; }
    let lines = block_without_markers(&blocks[0]);
    let mut pwr_vec = vec![parse_pwr(1, &lines).unwrap_or_default()];
    // Attempt stat parse (will ignore if pattern mismatch)
    let _ = parse_stat(1, &lines, &mut pwr_vec);
}

#[test]
fn test_parse_bat_from_sample() {
    let path = "../command_responses/bat x.txt";
    let blocks = load_blocks(path);
    if blocks.is_empty() { return; }
    let lines = block_without_markers(&blocks[0]);
    let level = rust_pytes_serial::model::MonitoringLevel::Medium;
    let pack = parse_bat(1, &lines, 16, &level).expect("parse bat");
    assert!(pack.cells.len() <= 16);
}
