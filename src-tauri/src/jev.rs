use crate::model::*;
use reqwest::StatusCode;
use serde_json::{json, Map, Value};
use std::{cmp::Ordering, collections::BTreeMap, time::Duration};

const API_URL: &str = "https://api.typesafe.ai/v1/systemone";
const MODEL: &str = "jev-latest";
const KEYCHAIN_SERVICE: &str = "com.omavashia.jev-mac-doctor.typesafe";
const KEYCHAIN_ACCOUNT: &str = "api-key";
// TypeSafe enforces token rather than byte limits. Telemetry JSON contains many
// short keys and numeric values, so use a deliberately conservative byte ceiling
// instead of assuming the usual prose-oriented bytes-per-token ratio.
const MAX_REQUEST_BYTES: usize = 48_000;

const SEVERITY_LEVELS: [&str; 10] = [
    "Effectively no slowdown",
    "Extremely slight slowdown",
    "Slight slowdown",
    "Mild slowdown",
    "Moderate slowdown",
    "Noticeable slowdown",
    "Substantial slowdown",
    "Severe slowdown",
    "Very severe slowdown",
    "Near-unusable slowdown",
];

const MECHANISMS: [(&str, &str); 8] = [
    ("cpu_contention", "CPU saturation or competing CPU work"),
    (
        "memory_pressure",
        "Memory pressure, footprint growth, compression, paging, or swap",
    ),
    ("disk_io", "Storage reads, writes, or operation pressure"),
    ("gpu_contention", "GPU execution or system GPU contention"),
    ("network_activity", "Observed network traffic pressure"),
    (
        "scheduler_pressure",
        "Runnable time, waits, wakeups, forced idle, or QoS effects",
    ),
    (
        "thermal_power",
        "Thermal pressure, throttling, frequency, or power constraints",
    ),
    (
        "no_clear_mechanism",
        "No listed mechanism is clearly supported by available telemetry",
    ),
];

pub struct JevRequest {
    pub body: Value,
    relevant: Vec<ProcessIdentity>,
}

pub fn credential_source() -> Option<&'static str> {
    if keychain_api_key().is_some() {
        Some("macOS Keychain")
    } else if environment_api_key().is_some() {
        Some("environment")
    } else {
        None
    }
}

pub fn save_api_key(api_key: &str) -> Result<(), String> {
    let api_key = api_key.trim();
    if api_key.is_empty() || api_key.len() > 4_096 {
        return Err("TypeSafe API key must contain between 1 and 4096 characters".into());
    }
    security_framework::passwords::set_generic_password(
        KEYCHAIN_SERVICE,
        KEYCHAIN_ACCOUNT,
        api_key.as_bytes(),
    )
    .map_err(|_| "failed to save the TypeSafe API key in macOS Keychain".to_owned())
}

fn keychain_api_key() -> Option<String> {
    security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|key| !key.trim().is_empty())
}

fn environment_api_key() -> Option<String> {
    std::env::var("TYPESAFE_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
}

fn api_key() -> Result<String, String> {
    keychain_api_key()
        .or_else(environment_api_key)
        .ok_or_else(|| {
            "TypeSafe API key is not configured; event was retained for later analysis".to_owned()
        })
}

pub fn build_request(event: &SlowdownEvent) -> Result<JevRequest, String> {
    let mut max_processes = 12;
    let mut stride = 1;
    loop {
        let relevant = rank_processes(&event.processes, max_processes);
        let state = build_state(event, &relevant, stride);
        let questions = build_questions(&relevant);
        let body = json!({ "state": state, "model": MODEL, "questions": questions });
        let bytes = serde_json::to_vec(&body)
            .map_err(|error| format!("failed to serialize Jev request: {error}"))?
            .len();
        if bytes <= MAX_REQUEST_BYTES {
            return Ok(JevRequest { body, relevant });
        }
        if max_processes > 6 {
            max_processes -= 2;
        } else if stride < 8 {
            stride += 1;
        } else {
            return Err(format!(
                "Jev request is {bytes} bytes after explicit compaction; refusing to silently drop more telemetry"
            ));
        }
    }
}

pub async fn evaluate(request: &JevRequest) -> Result<JevSlowdownResult, String> {
    let api_key = api_key()?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| format!("failed to initialize TypeSafe client: {error}"))?;
    let mut delay = Duration::from_secs(1);
    for attempt in 0..3 {
        let response = client
            .post(API_URL)
            .bearer_auth(&api_key)
            .json(&request.body)
            .send()
            .await
            .map_err(|error| format!("TypeSafe request failed: {error}"))?;
        let status = response.status();
        if status.is_success() {
            let raw = response
                .json::<Value>()
                .await
                .map_err(|error| format!("TypeSafe returned invalid JSON: {error}"))?;
            return normalize(raw, &request.relevant);
        }
        let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.as_u16() == 529;
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "response body unavailable".into());
        if !retryable || attempt == 2 {
            return Err(format!(
                "TypeSafe API returned {status}: {}",
                message.chars().take(500).collect::<String>()
            ));
        }
        tokio::time::sleep(delay).await;
        delay *= 2;
    }
    Err("TypeSafe retry loop ended unexpectedly".into())
}

fn build_state(event: &SlowdownEvent, relevant: &[ProcessIdentity], stride: usize) -> Value {
    let selected_ids = relevant
        .iter()
        .map(|identity| identity.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let processes = event
        .processes
        .iter()
        .filter(|window| selected_ids.contains(window.identity.id.as_str()))
        .map(|window| {
            let samples = window
                .samples
                .iter()
                .step_by(stride)
                .map(compact_process_sample)
                .collect::<Vec<_>>();
            json!({ "identity": window.identity, "samples": samples })
        })
        .collect::<Vec<_>>();
    let system_samples = event
        .system_samples
        .iter()
        .step_by(stride)
        .map(compact_system_sample)
        .collect::<Vec<_>>();
    json!({
        "event": {
            "type": "user_reported_slowdown",
            "timestamp_ns": event.clicked_at_ns,
            "window_start_ns": event.window_start_ns,
            "window_end_ns": event.window_end_ns,
            "window_before_seconds": 10,
            "window_after_seconds": 10
        },
        "capabilities": event.capabilities,
        "compaction": {
            "full_process_count": event.processes.len(),
            "sent_process_count": processes.len(),
            "sample_stride": stride,
            "selection": "highest observed CPU, GPU, disk, wakeups, and memory signal",
            "full_event_retained_locally": true
        },
        "system_samples": system_samples,
        "processes": processes
    })
}

fn compact_process_sample(sample: &ProcessSample) -> Value {
    json!({
        "timestamp_ns": sample.timestamp_ns,
        "cpu_percent": sample.cpu.percent,
        "instructions_per_second": sample.cpu.instructions_per_second,
        "cycles_per_second": sample.cpu.cycles_per_second,
        "runnable_time_ns": sample.cpu.runnable_time_ns,
        "gpu_utilization_percent": sample.gpu.utilization_percent,
        "memory_physical_footprint_bytes": sample.memory.physical_footprint_bytes,
        "memory_resident_bytes": sample.memory.resident_bytes,
        "disk_read_bytes_per_second": sample.disk_io.read_bytes_per_second,
        "disk_write_bytes_per_second": sample.disk_io.write_bytes_per_second,
        "network_available": sample.network.available,
        "network_rx_bytes_per_second": sample.network.rx_bytes_per_second,
        "network_tx_bytes_per_second": sample.network.tx_bytes_per_second,
        "interrupt_wakeups_per_second": sample.scheduling.interrupt_wakeups_per_second,
        "package_idle_wakeups_per_second": sample.scheduling.package_idle_wakeups_per_second,
        "wait_time_ns": sample.scheduling.wait_time_ns,
        "energy_nj": sample.energy.energy_nj,
        "p_core_energy_nj": sample.energy.p_core_energy_nj,
        "neural_footprint_bytes": sample.neural.footprint_bytes
    })
}

fn compact_system_sample(sample: &SystemSample) -> Value {
    json!({
        "timestamp_ns": sample.timestamp_ns,
        "cpu": sample.cpu,
        "gpu": sample.gpu,
        "ane": sample.ane,
        "memory": sample.memory,
        "disk": sample.disk,
        "network": sample.network,
        "scheduler": sample.scheduler,
        "thermal": sample.thermal
    })
}

fn build_questions(relevant: &[ProcessIdentity]) -> Value {
    let mut questions = Map::new();
    questions.insert(
        "slowdown_severity".into(),
        json!({
            "type": "score",
            "instructions": "Based on the provided 20-second system/process telemetry surrounding the explicit user-reported slowdown timestamp, how severe does the observed slowdown appear? Judge only observable degradation signals and treat unavailable metrics as unknown, not zero.",
            "criteria": SEVERITY_LEVELS
        }),
    );
    let mut primary = Map::new();
    for (index, identity) in relevant.iter().enumerate() {
        primary.insert(
            format!("p{index}"),
            Value::String(format!(
                "{} (PID {}, stable identity {})",
                identity.name, identity.pid, identity.id
            )),
        );
    }
    primary.insert(
        "system_or_unknown".into(),
        Value::String(
            "System-wide, kernel, unavailable telemetry, or no listed process explains it".into(),
        ),
    );
    questions.insert(
        "primary_cause".into(),
        json!({
            "type": "choice",
            "instructions": "Which listed process provides the strongest explanation for the user-observed slowdown around the marked timestamp, given the complete telemetry window?",
            "criteria": primary
        }),
    );
    let mechanism_criteria = MECHANISMS
        .into_iter()
        .map(|(key, description)| (key.to_owned(), Value::String(description.to_owned())))
        .collect::<Map<_, _>>();
    for (index, identity) in relevant.iter().enumerate() {
        questions.insert(
            format!("contribution_{index}"),
            json!({
                "type": "noul",
                "instructions": format!("Did process {} (PID {}, stable identity {}) materially contribute to the slowdown observed around the marked timestamp?", identity.name, identity.pid, identity.id),
                "criteria": {
                    "true": "Its observed telemetry materially supports a causal contribution",
                    "false": "Its telemetry is normal, irrelevant, unavailable, or does not materially support contribution"
                }
            }),
        );
        questions.insert(
            format!("mechanism_{index}"),
            json!({
                "type": "choice",
                "instructions": format!("If process {} (PID {}, stable identity {}) contributed to the slowdown, which listed mechanism in the observed telemetry provides the strongest explanation? Treat unavailable metrics as unknown and do not infer an unobserved mechanism.", identity.name, identity.pid, identity.id),
                "criteria": mechanism_criteria
            }),
        );
    }
    Value::Object(questions)
}

fn rank_processes(windows: &[ProcessWindow], limit: usize) -> Vec<ProcessIdentity> {
    let mut scored = windows
        .iter()
        .map(|window| {
            let score = window.samples.iter().fold(0.0_f64, |score, sample| {
                score.max(
                    sample.cpu.percent.unwrap_or_default()
                        + sample.gpu.utilization_percent.unwrap_or_default() * 1.5
                        + sample.disk_io.read_bytes_per_second.unwrap_or_default() / 1_000_000.0
                        + sample.disk_io.write_bytes_per_second.unwrap_or_default() / 1_000_000.0
                        + sample
                            .scheduling
                            .interrupt_wakeups_per_second
                            .unwrap_or_default()
                            / 100.0
                        + sample.memory.physical_footprint_bytes.unwrap_or_default() as f64
                            / 100_000_000.0,
                )
            });
            (score, window.identity.clone())
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, identity)| identity)
        .collect()
}

pub fn normalize(raw: Value, relevant: &[ProcessIdentity]) -> Result<JevSlowdownResult, String> {
    let answers = raw
        .get("answers")
        .and_then(Value::as_object)
        .ok_or_else(|| "TypeSafe response is missing answers".to_owned())?;
    let severity = answers
        .get("slowdown_severity")
        .and_then(Value::as_object)
        .ok_or_else(|| "TypeSafe response is missing slowdown_severity".to_owned())?;
    let severity_probabilities = number_map(severity.get("probabilities"))?;
    let legend = SEVERITY_LEVELS
        .iter()
        .enumerate()
        .map(|(index, description)| (index.to_string(), (*description).to_owned()))
        .collect();
    let primary = answers
        .get("primary_cause")
        .and_then(Value::as_object)
        .ok_or_else(|| "TypeSafe response is missing primary_cause".to_owned())?;
    let map_primary_key = |key: &str| -> String {
        key.strip_prefix('p')
            .and_then(|index| index.parse::<usize>().ok())
            .and_then(|index| relevant.get(index))
            .map(|identity| identity.id.clone())
            .unwrap_or_else(|| "system_or_unknown".into())
    };
    let primary_probabilities = number_map(primary.get("probabilities"))?
        .into_iter()
        .map(|(key, probability)| (map_primary_key(&key), probability))
        .collect();
    let primary_choice = primary
        .get("choice")
        .and_then(Value::as_str)
        .map(map_primary_key)
        .ok_or_else(|| "primary_cause choice is missing".to_owned())?;
    let mut process_results = Vec::with_capacity(relevant.len());
    for (index, identity) in relevant.iter().enumerate() {
        let contribution = answers
            .get(&format!("contribution_{index}"))
            .and_then(|answer| answer.get("noul"))
            .and_then(Value::as_f64);
        let mechanism = answers
            .get(&format!("mechanism_{index}"))
            .and_then(Value::as_object)
            .map(|answer| {
                Ok::<_, String>(JevMechanism {
                    choice: answer
                        .get("choice")
                        .and_then(Value::as_str)
                        .ok_or_else(|| format!("mechanism_{index} choice is missing"))?
                        .to_owned(),
                    confidence: answer.get("confidence").and_then(Value::as_f64),
                    probabilities: number_map(answer.get("probabilities"))?,
                })
            })
            .transpose()?;
        process_results.push(JevProcessResult {
            process_id: identity.id.clone(),
            pid: identity.pid,
            name: identity.name.clone(),
            contribution_probability: contribution,
            mechanism,
        });
    }
    Ok(JevSlowdownResult {
        slowdown_severity: JevSeverity {
            score: severity
                .get("score")
                .and_then(Value::as_f64)
                .ok_or_else(|| "slowdown_severity score is missing".to_owned())?,
            confidence: severity.get("confidence").and_then(Value::as_f64),
            probabilities: severity_probabilities,
            legend,
        },
        primary_cause: JevPrimaryCause {
            process_id: primary_choice,
            confidence: primary.get("confidence").and_then(Value::as_f64),
            probabilities: primary_probabilities,
        },
        processes: process_results,
        raw_response: raw,
    })
}

fn number_map(value: Option<&Value>) -> Result<BTreeMap<String, f64>, String> {
    value
        .and_then(Value::as_object)
        .ok_or_else(|| "expected a probability map".to_owned())?
        .iter()
        .map(|(key, value)| {
            value
                .as_f64()
                .map(|number| (key.clone(), number))
                .ok_or_else(|| format!("probability {key} is not numeric"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ProcessIdentity {
        ProcessIdentity {
            id: "42:100.000001".into(),
            pid: 42,
            name: "worker".into(),
            ..Default::default()
        }
    }

    #[test]
    fn normalizes_all_three_primitives() {
        let mechanisms = MECHANISMS
            .iter()
            .map(|(key, _)| {
                (
                    (*key).to_owned(),
                    json!(if *key == "cpu_contention" { 1.0 } else { 0.0 }),
                )
            })
            .collect::<Map<_, _>>();
        let raw = json!({
            "answers": {
                "slowdown_severity": {"type":"score","score":6.0,"confidence":0.8,"probabilities":{"6":1.0}},
                "primary_cause": {"type":"choice","choice":"p0","confidence":0.9,"probabilities":{"p0":0.9,"system_or_unknown":0.1}},
                "contribution_0": {"type":"noul","noul":0.87},
                "mechanism_0": {"type":"choice","choice":"cpu_contention","confidence":1.0,"probabilities": mechanisms}
            }
        });
        let result = normalize(raw, &[identity()]).unwrap();
        assert_eq!(result.primary_cause.process_id, "42:100.000001");
        assert_eq!(result.processes[0].contribution_probability, Some(0.87));
        assert_eq!(
            result.processes[0].mechanism.as_ref().unwrap().choice,
            "cpu_contention"
        );
    }

    #[test]
    fn serializes_bounded_state_and_typed_questions() {
        let event = SlowdownEvent {
            id: "event".into(),
            clicked_at_ns: 20,
            window_start_ns: 10,
            window_end_ns: 30,
            status: SlowdownStatus::BuildingState,
            system_samples: vec![SystemSample {
                timestamp_ns: 20,
                ..Default::default()
            }],
            processes: vec![ProcessWindow {
                identity: identity(),
                samples: vec![ProcessSample {
                    timestamp_ns: 20,
                    identity: identity(),
                    ..Default::default()
                }],
            }],
            capabilities: BTreeMap::new(),
            jev_request: None,
            jev_result: None,
            errors: vec![],
        };
        let request = build_request(&event).unwrap();
        assert_eq!(request.body["model"], MODEL);
        assert_eq!(
            request.body["questions"]["slowdown_severity"]["type"],
            "score"
        );
        assert_eq!(request.body["questions"]["primary_cause"]["type"], "choice");
        assert_eq!(request.body["questions"]["contribution_0"]["type"], "noul");
        assert!(serde_json::to_vec(&request.body).unwrap().len() <= MAX_REQUEST_BYTES);
    }

    #[test]
    fn compacts_a_dense_window_below_the_jev_limit() {
        let mut event = SlowdownEvent {
            id: "event".into(),
            clicked_at_ns: 20_000_000_000,
            window_start_ns: 10_000_000_000,
            window_end_ns: 30_000_000_000,
            status: SlowdownStatus::BuildingState,
            system_samples: (0..21)
                .map(|second| SystemSample {
                    timestamp_ns: (10 + second) * 1_000_000_000,
                    ..Default::default()
                })
                .collect(),
            processes: vec![],
            capabilities: BTreeMap::new(),
            jev_request: None,
            jev_result: None,
            errors: vec![],
        };
        for pid in 1..=30 {
            let identity = ProcessIdentity {
                id: format!("{pid}:100.000001"),
                pid,
                name: format!("process-{pid}"),
                ..Default::default()
            };
            event.processes.push(ProcessWindow {
                identity: identity.clone(),
                samples: (0..21)
                    .map(|second| ProcessSample {
                        timestamp_ns: (10 + second) * 1_000_000_000,
                        identity: identity.clone(),
                        ..Default::default()
                    })
                    .collect(),
            });
        }

        let request = build_request(&event).unwrap();
        assert!(serde_json::to_vec(&request.body).unwrap().len() <= MAX_REQUEST_BYTES);
        assert_eq!(
            request.body["state"]["compaction"]["full_process_count"],
            30
        );
        assert!(
            request.body["state"]["compaction"]["sent_process_count"]
                .as_u64()
                .unwrap()
                < 30
        );
    }
}
