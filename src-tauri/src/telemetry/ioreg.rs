use plist::{Dictionary, Value};
use regex::Regex;
use std::{collections::HashMap, io::Cursor, process::Command, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default)]
pub struct DiskCounters {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

fn read_registry_class(class_name: &str) -> Result<Value, String> {
    // Read-only safety: `/usr/sbin/ioreg -a -r -c <class>` serializes registry
    // properties as a plist. The fixed executable and arguments contain no
    // write/control operation and are never supplied by the frontend.
    let output = Command::new("/usr/sbin/ioreg")
        .args(["-a", "-r", "-c", class_name])
        .output()
        .map_err(|error| format!("failed to start ioreg: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Value::from_reader(Cursor::new(output.stdout))
        .map_err(|error| format!("invalid ioreg plist: {error}"))
}

pub fn agx_gpu_times() -> Result<HashMap<i32, u64>, String> {
    let root = read_registry_class("AGXAccelerator")?;
    let mut result = HashMap::new();
    walk_agx(&root, None, &mut result);
    if result.is_empty() {
        return Err("AGX AppUsage/accumulatedGPUTime schema was not found".into());
    }
    Ok(result)
}

fn walk_agx(value: &Value, inherited_pid: Option<i32>, result: &mut HashMap<i32, u64>) {
    match value {
        Value::Array(values) => {
            for value in values {
                walk_agx(value, inherited_pid, result);
            }
        }
        Value::Dictionary(dict) => {
            let pid = pid_from_dictionary(dict).or(inherited_pid);
            if let (Some(pid), Some(time)) = (pid, integer(dict, "accumulatedGPUTime")) {
                result
                    .entry(pid)
                    .and_modify(|current| *current = (*current).max(time))
                    .or_insert(time);
            }
            for value in dict.values() {
                walk_agx(value, pid, result);
            }
        }
        _ => {}
    }
}

fn pid_from_dictionary(dict: &Dictionary) -> Option<i32> {
    for key in ["pid", "PID", "IOUserClientCreatorPID"] {
        if let Some(pid) = dict.get(key).and_then(as_u64) {
            return i32::try_from(pid).ok();
        }
    }
    dict.get("IOUserClientCreator")
        .and_then(Value::as_string)
        .and_then(pid_from_creator)
}

fn pid_from_creator(value: &str) -> Option<i32> {
    static PID: OnceLock<Regex> = OnceLock::new();
    PID.get_or_init(|| Regex::new(r"(?i)\bpid\D{0,4}(\d+)\b").expect("valid PID regex"))
        .captures(value)
        .and_then(|capture| capture.get(1))
        .and_then(|number| number.as_str().parse().ok())
}

pub fn storage_counters() -> Result<DiskCounters, String> {
    let root = read_registry_class("IOBlockStorageDriver")?;
    let mut counters = DiskCounters::default();
    let mut found = false;
    walk_storage(&root, &mut counters, &mut found);
    found
        .then_some(counters)
        .ok_or_else(|| "IOBlockStorageDriver Statistics property was not found".into())
}

fn walk_storage(value: &Value, counters: &mut DiskCounters, found: &mut bool) {
    match value {
        Value::Array(values) => {
            for value in values {
                walk_storage(value, counters, found);
            }
        }
        Value::Dictionary(dict) => {
            if let Some(Value::Dictionary(stats)) = dict.get("Statistics") {
                *found = true;
                counters.read_bytes = counters
                    .read_bytes
                    .saturating_add(integer(stats, "Bytes (Read)").unwrap_or_default());
                counters.write_bytes = counters
                    .write_bytes
                    .saturating_add(integer(stats, "Bytes (Write)").unwrap_or_default());
                counters.read_ops = counters
                    .read_ops
                    .saturating_add(integer(stats, "Operations (Read)").unwrap_or_default());
                counters.write_ops = counters
                    .write_ops
                    .saturating_add(integer(stats, "Operations (Write)").unwrap_or_default());
            }
            for value in dict.values() {
                walk_storage(value, counters, found);
            }
        }
        _ => {}
    }
}

fn integer(dict: &Dictionary, key: &str) -> Option<u64> {
    dict.get(key).and_then(as_u64)
}

fn as_u64(value: &Value) -> Option<u64> {
    value.as_unsigned_integer().or_else(|| {
        value
            .as_signed_integer()
            .and_then(|number| u64::try_from(number).ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agx_creator_and_gpu_time() {
        let value = Value::Dictionary(Dictionary::from_iter([(
            "AppUsage".to_owned(),
            Value::Array(vec![Value::Dictionary(Dictionary::from_iter([
                (
                    "IOUserClientCreator".to_owned(),
                    Value::String("pid 4242, Test App".into()),
                ),
                (
                    "accumulatedGPUTime".to_owned(),
                    Value::Integer(9000_u64.into()),
                ),
            ]))]),
        )]));
        let mut parsed = HashMap::new();
        walk_agx(&value, None, &mut parsed);
        assert_eq!(parsed.get(&4242), Some(&9000));
    }

    #[test]
    fn parses_storage_statistics() {
        let value = Value::Dictionary(Dictionary::from_iter([(
            "Statistics".to_owned(),
            Value::Dictionary(Dictionary::from_iter([
                ("Bytes (Read)".to_owned(), Value::Integer(10_u64.into())),
                ("Bytes (Write)".to_owned(), Value::Integer(20_u64.into())),
                ("Operations (Read)".to_owned(), Value::Integer(2_u64.into())),
                (
                    "Operations (Write)".to_owned(),
                    Value::Integer(3_u64.into()),
                ),
            ])),
        )]));
        let mut counters = DiskCounters::default();
        let mut found = false;
        walk_storage(&value, &mut counters, &mut found);
        assert!(found);
        assert_eq!(counters.read_bytes, 10);
        assert_eq!(counters.write_ops, 3);
    }
}
