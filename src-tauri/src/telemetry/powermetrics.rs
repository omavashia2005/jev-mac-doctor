use plist::Value;
use std::{io::Cursor, process::Command};

#[derive(Clone, Copy, Debug, Default)]
pub struct Detection {
    pub installed: bool,
    pub plist_supported: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PowerSample {
    pub cpu_power_mw: Option<f64>,
    pub gpu_power_mw: Option<f64>,
    pub ane_power_mw: Option<f64>,
    pub gpu_active_percent: Option<f64>,
    pub ane_active_percent: Option<f64>,
    pub forced_idle_percent: Option<f64>,
}

pub fn detect() -> Detection {
    // Read-only safety: `powermetrics --help` only reports supported options.
    // It does not start a sampler, request privilege, or alter power state.
    let output = Command::new("/usr/bin/powermetrics").arg("--help").output();
    let Ok(output) = output else {
        return Detection::default();
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Detection {
        installed: output.status.success(),
        plist_supported: text.contains("plist") && text.contains("--format"),
    }
}

#[allow(dead_code)]
pub fn parse_plist_sample(bytes: &[u8]) -> Result<PowerSample, String> {
    let bytes = bytes
        .split(|byte| *byte == 0)
        .find(|part| !part.is_empty())
        .unwrap_or(bytes);
    let root = Value::from_reader(Cursor::new(bytes))
        .map_err(|error| format!("invalid powermetrics plist sample: {error}"))?;
    Ok(PowerSample {
        cpu_power_mw: find_number(&root, &["cpu_power_mw", "cpu power"]),
        gpu_power_mw: find_number(&root, &["gpu_power_mw", "gpu power"]),
        ane_power_mw: find_number(&root, &["ane_power_mw", "ane power"]),
        gpu_active_percent: find_number(&root, &["gpu_active_percent", "gpu active"]),
        ane_active_percent: find_number(&root, &["ane_active_percent", "ane active"]),
        forced_idle_percent: find_number(&root, &["forced_idle_percent", "forced idle"]),
    })
}

#[allow(dead_code)]
fn find_number(value: &Value, keys: &[&str]) -> Option<f64> {
    match value {
        Value::Dictionary(dictionary) => {
            for (key, value) in dictionary {
                let normalized = key.to_ascii_lowercase().replace(['-', '_'], " ");
                if keys
                    .iter()
                    .any(|candidate| normalized == candidate.replace('_', " "))
                {
                    if let Some(number) = value.as_real() {
                        return Some(number);
                    }
                    if let Some(number) = value.as_unsigned_integer() {
                        return Some(number as f64);
                    }
                    if let Some(number) = value.as_signed_integer() {
                        return Some(number as f64);
                    }
                }
                if let Some(number) = find_number(value, keys) {
                    return Some(number);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(|value| find_number(value, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_power_sample_and_keeps_missing_fields_none() {
        let fixture = br#"<?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0"><dict>
          <key>processor</key><dict><key>CPU Power</key><real>1250.5</real></dict>
          <key>graphics</key><dict><key>GPU Active Percent</key><real>42.0</real></dict>
        </dict></plist>"#;
        let sample = parse_plist_sample(fixture).unwrap();
        assert_eq!(sample.cpu_power_mw, Some(1250.5));
        assert_eq!(sample.gpu_active_percent, Some(42.0));
        assert_eq!(sample.ane_power_mw, None);
    }
}
