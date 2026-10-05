mod jev;
mod model;
mod persistence;
mod telemetry;

use crate::{model::*, persistence::EventStore, telemetry::TelemetryCollector};
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use uuid::Uuid;

const DEFAULT_SAMPLE_INTERVAL_MS: u64 = 1_000;
const RETENTION_NS: u64 = 60_000_000_000;
const EVENT_BEFORE_NS: u64 = 10_000_000_000;
const EVENT_AFTER_NS: u64 = 10_000_000_000;

struct RuntimeState {
    buffer: VecDeque<TelemetryFrame>,
    capabilities: BTreeMap<String, Capability>,
    errors: VecDeque<DiagnosticError>,
    running: bool,
}

#[derive(Clone)]
struct AppState {
    runtime: Arc<Mutex<RuntimeState>>,
    interval_ms: Arc<AtomicU64>,
    store: EventStore,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureStarted {
    id: String,
    clicked_at_ns: u64,
    window_end_ns: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureStatus {
    id: String,
    clicked_at_ns: u64,
    window_end_ns: u64,
    status: SlowdownStatus,
    remaining_seconds: Option<u64>,
    message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeSafeCredentialStatus {
    configured: bool,
    source: Option<&'static str>,
}

#[tauri::command]
fn get_typesafe_credential_status() -> TypeSafeCredentialStatus {
    let source = jev::credential_source();
    TypeSafeCredentialStatus {
        configured: source.is_some(),
        source,
    }
}

#[tauri::command]
fn set_typesafe_api_key(api_key: String) -> Result<TypeSafeCredentialStatus, String> {
    jev::save_api_key(&api_key)?;
    Ok(get_typesafe_credential_status())
}

#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<CollectorSnapshot, String> {
    let runtime = state.runtime.lock().await;
    let latest = runtime.buffer.back().cloned();
    let processes_tracked = latest
        .as_ref()
        .map(|frame| frame.processes.len())
        .unwrap_or_default();
    Ok(CollectorSnapshot {
        running: runtime.running,
        sample_interval_ms: state.interval_ms.load(Ordering::Relaxed),
        processes_tracked,
        samples_retained: runtime.buffer.len(),
        last_sample_timestamp_ns: latest.as_ref().map(|frame| frame.timestamp_ns),
        latest: latest.map(public_frame),
        capabilities: runtime.capabilities.clone(),
        errors: runtime.errors.iter().cloned().collect(),
    })
}

#[tauri::command]
async fn get_capabilities(
    state: State<'_, AppState>,
) -> Result<BTreeMap<String, Capability>, String> {
    Ok(state.runtime.lock().await.capabilities.clone())
}

#[tauri::command]
async fn set_sample_interval(state: State<'_, AppState>, milliseconds: u64) -> Result<(), String> {
    if !(500..=5_000).contains(&milliseconds) {
        return Err("sample interval must be between 500 and 5000 ms".into());
    }
    state.interval_ms.store(milliseconds, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
async fn mark_slowdown(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CaptureStarted, String> {
    let clicked_at_ns = telemetry::now_ns();
    let window_start_ns = clicked_at_ns.saturating_sub(EVENT_BEFORE_NS);
    let window_end_ns = clicked_at_ns.saturating_add(EVENT_AFTER_NS);
    let id = Uuid::new_v4().to_string();
    let (pre_window, capabilities, errors) = {
        let runtime = state.runtime.lock().await;
        if !runtime.running {
            return Err("telemetry collector has not produced a sample yet".into());
        }
        (
            frames_in_window(&runtime.buffer, window_start_ns, clicked_at_ns),
            runtime.capabilities.clone(),
            runtime.errors.iter().cloned().collect::<Vec<_>>(),
        )
    };
    let started = CaptureStarted {
        id: id.clone(),
        clicked_at_ns,
        window_end_ns,
    };
    emit_status(
        &app,
        CaptureStatus {
            id: id.clone(),
            clicked_at_ns,
            window_end_ns,
            status: SlowdownStatus::CollectingPostWindow,
            remaining_seconds: Some(10),
            message: None,
        },
    );
    let app_state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        capture_event(
            app,
            app_state,
            id,
            clicked_at_ns,
            window_start_ns,
            window_end_ns,
            pre_window,
            capabilities,
            errors,
        )
        .await;
    });
    Ok(started)
}

#[tauri::command]
async fn list_events(state: State<'_, AppState>) -> Result<Vec<EventSummary>, String> {
    state.store.list()
}

#[tauri::command]
async fn get_event(state: State<'_, AppState>, id: String) -> Result<SlowdownEvent, String> {
    state.store.get(&id)
}

#[allow(clippy::too_many_arguments)]
async fn capture_event(
    app: AppHandle,
    state: AppState,
    id: String,
    clicked_at_ns: u64,
    window_start_ns: u64,
    window_end_ns: u64,
    mut frames: Vec<TelemetryFrame>,
    capabilities: BTreeMap<String, Capability>,
    mut errors: Vec<DiagnosticError>,
) {
    for remaining in (1..=10).rev() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        emit_status(
            &app,
            CaptureStatus {
                id: id.clone(),
                clicked_at_ns,
                window_end_ns,
                status: SlowdownStatus::CollectingPostWindow,
                remaining_seconds: Some(remaining - 1),
                message: None,
            },
        );
    }
    {
        let runtime = state.runtime.lock().await;
        frames.extend(frames_in_window(
            &runtime.buffer,
            clicked_at_ns.saturating_add(1),
            window_end_ns,
        ));
        errors.extend(
            runtime
                .errors
                .iter()
                .filter(|error| {
                    error.timestamp_ns >= window_start_ns && error.timestamp_ns <= window_end_ns
                })
                .cloned(),
        );
    }
    frames.sort_by_key(|frame| frame.timestamp_ns);
    frames.dedup_by_key(|frame| frame.timestamp_ns);
    let mut event = event_from_frames(
        id.clone(),
        clicked_at_ns,
        window_start_ns,
        window_end_ns,
        frames,
        capabilities,
        errors,
    );
    event.status = SlowdownStatus::BuildingState;
    emit_status(
        &app,
        CaptureStatus {
            id: id.clone(),
            clicked_at_ns,
            window_end_ns,
            status: event.status.clone(),
            remaining_seconds: None,
            message: None,
        },
    );
    match jev::build_request(&event) {
        Ok(request) => {
            event.jev_request = Some(request.body.clone());
            event.status = SlowdownStatus::QueryingJev;
            emit_status(
                &app,
                CaptureStatus {
                    id: id.clone(),
                    clicked_at_ns,
                    window_end_ns,
                    status: event.status.clone(),
                    remaining_seconds: None,
                    message: None,
                },
            );
            match jev::evaluate(&request).await {
                Ok(result) => {
                    event.jev_result = Some(result);
                    event.status = SlowdownStatus::Complete;
                }
                Err(message) => {
                    event.errors.push(telemetry::diagnostic_error(
                        telemetry::now_ns(),
                        "jev",
                        message.clone(),
                        true,
                    ));
                    event.status = SlowdownStatus::Failed;
                    emit_status(
                        &app,
                        CaptureStatus {
                            id: id.clone(),
                            clicked_at_ns,
                            window_end_ns,
                            status: event.status.clone(),
                            remaining_seconds: None,
                            message: Some(message),
                        },
                    );
                }
            }
        }
        Err(message) => {
            event.errors.push(telemetry::diagnostic_error(
                telemetry::now_ns(),
                "jev-state",
                message,
                false,
            ));
            event.status = SlowdownStatus::Failed;
        }
    }
    if let Err(message) = state.store.save(&event) {
        event.errors.push(telemetry::diagnostic_error(
            telemetry::now_ns(),
            "persistence",
            message,
            true,
        ));
    }
    let _ = app.emit("slowdown://completed", &event);
    emit_status(
        &app,
        CaptureStatus {
            id,
            clicked_at_ns,
            window_end_ns,
            status: event.status,
            remaining_seconds: None,
            message: event.errors.last().map(|error| error.message.clone()),
        },
    );
}

fn event_from_frames(
    id: String,
    clicked_at_ns: u64,
    window_start_ns: u64,
    window_end_ns: u64,
    frames: Vec<TelemetryFrame>,
    capabilities: BTreeMap<String, Capability>,
    errors: Vec<DiagnosticError>,
) -> SlowdownEvent {
    let system_samples = frames.iter().map(|frame| frame.system.clone()).collect();
    let mut processes = BTreeMap::<String, ProcessWindow>::new();
    for sample in frames.into_iter().flat_map(|frame| frame.processes) {
        processes
            .entry(sample.identity.id.clone())
            .or_insert_with(|| ProcessWindow {
                identity: sample.identity.clone(),
                samples: Vec::new(),
            })
            .samples
            .push(sample);
    }
    SlowdownEvent {
        id,
        clicked_at_ns,
        window_start_ns,
        window_end_ns,
        status: SlowdownStatus::CollectingPostWindow,
        system_samples,
        processes: processes.into_values().collect(),
        capabilities,
        jev_request: None,
        jev_result: None,
        errors,
    }
}

fn frames_in_window(
    buffer: &VecDeque<TelemetryFrame>,
    start_ns: u64,
    end_ns: u64,
) -> Vec<TelemetryFrame> {
    buffer
        .iter()
        .filter(|frame| frame.timestamp_ns >= start_ns && frame.timestamp_ns <= end_ns)
        .cloned()
        .collect()
}

fn public_frame(mut frame: TelemetryFrame) -> TelemetryFrame {
    frame.processes.sort_by(|a, b| {
        b.cpu
            .percent
            .unwrap_or_default()
            .partial_cmp(&a.cpu.percent.unwrap_or_default())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    frame.processes.truncate(100);
    frame
}

fn emit_status(app: &AppHandle, status: CaptureStatus) {
    let _ = app.emit("slowdown://capture-status", status);
}

fn start_sampler(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut collector = TelemetryCollector::new();
        loop {
            match collector.sample() {
                Ok(frame) => {
                    let public = public_frame(frame.clone());
                    let capabilities = collector.capabilities();
                    {
                        let mut runtime = state.runtime.lock().await;
                        runtime.running = true;
                        runtime.capabilities = capabilities.clone();
                        runtime.buffer.push_back(frame);
                        let cutoff = telemetry::now_ns().saturating_sub(RETENTION_NS);
                        trim_buffer(&mut runtime.buffer, cutoff);
                    }
                    let _ = app.emit("telemetry://sample", public);
                    let _ = app.emit("telemetry://capabilities", capabilities);
                }
                Err(errors) => {
                    let mut runtime = state.runtime.lock().await;
                    for error in errors {
                        let _ = app.emit("collector://error", &error);
                        record_error(&mut runtime, error);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(
                state.interval_ms.load(Ordering::Relaxed),
            ))
            .await;
        }
    });
}

fn trim_buffer(buffer: &mut VecDeque<TelemetryFrame>, cutoff_ns: u64) {
    while buffer
        .front()
        .is_some_and(|frame| frame.timestamp_ns < cutoff_ns)
    {
        buffer.pop_front();
    }
}

fn record_error(runtime: &mut RuntimeState, error: DiagnosticError) {
    runtime.errors.push_back(error);
    while runtime.errors.len() > 100 {
        runtime.errors.pop_front();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    tauri::Builder::default()
        .setup(|app| {
            let directory = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("failed to resolve app data directory: {error}"))?
                .join("events");
            let state = AppState {
                runtime: Arc::new(Mutex::new(RuntimeState {
                    buffer: VecDeque::new(),
                    capabilities: TelemetryCollector::new().capabilities(),
                    errors: VecDeque::new(),
                    running: false,
                })),
                interval_ms: Arc::new(AtomicU64::new(DEFAULT_SAMPLE_INTERVAL_MS)),
                store: EventStore::new(directory).map_err(std::io::Error::other)?,
            };
            app.manage(state.clone());
            start_sampler(app.handle().clone(), state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_capabilities,
            set_sample_interval,
            mark_slowdown,
            list_events,
            get_event,
            get_typesafe_credential_status,
            set_typesafe_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jev Mac Doctor");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(timestamp_ns: u64) -> TelemetryFrame {
        TelemetryFrame {
            timestamp_ns,
            system: SystemSample {
                timestamp_ns,
                ..Default::default()
            },
            processes: vec![],
        }
    }

    #[test]
    fn extracts_inclusive_slowdown_window() {
        let buffer = VecDeque::from([frame(9), frame(10), frame(15), frame(20), frame(21)]);
        let selected = frames_in_window(&buffer, 10, 20);
        assert_eq!(
            selected.iter().map(|f| f.timestamp_ns).collect::<Vec<_>>(),
            [10, 15, 20]
        );
    }

    #[test]
    fn event_starts_in_collecting_state() {
        let event = event_from_frames(
            "event".into(),
            20,
            10,
            30,
            vec![frame(20)],
            BTreeMap::new(),
            vec![],
        );
        assert!(matches!(event.status, SlowdownStatus::CollectingPostWindow));
        assert_eq!(event.system_samples.len(), 1);
    }

    #[test]
    fn rolling_buffer_retains_only_recent_samples() {
        let mut buffer = VecDeque::from([frame(10), frame(20), frame(30)]);
        trim_buffer(&mut buffer, 20);
        assert_eq!(
            buffer.iter().map(|f| f.timestamp_ns).collect::<Vec<_>>(),
            [20, 30]
        );
    }

    #[test]
    fn collector_error_does_not_discard_healthy_telemetry() {
        let mut runtime = RuntimeState {
            buffer: VecDeque::from([frame(10)]),
            capabilities: BTreeMap::new(),
            errors: VecDeque::new(),
            running: true,
        };
        record_error(
            &mut runtime,
            DiagnosticError {
                timestamp_ns: 11,
                collector: "optional".into(),
                message: "unavailable".into(),
                recoverable: true,
            },
        );
        assert_eq!(runtime.buffer.len(), 1);
        assert_eq!(runtime.errors.len(), 1);
    }
}
