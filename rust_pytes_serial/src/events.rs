use crate::model::{PowerData, Aggregate};
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct EventsSummary { pub bat_events_no: u64, pub pwr_events_no: u64, pub sys_events_no: u64 }

pub fn detect_events(pwr: &[PowerData], _agg: &Aggregate) -> EventsSummary {
    let mut summary = EventsSummary::default();
    for p in pwr {
        if let Some(v) = p.bat_events { if v !=0 { summary.bat_events_no += 1; } }
        if let Some(v) = p.power_events { if v !=0 { summary.pwr_events_no += 1; } }
        if let Some(v) = p.sys_events { if v !=0 { summary.sys_events_no += 1; } }
    }
    summary
}
