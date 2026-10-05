use crate::model::{EventSummary, SlowdownEvent};
use std::{fs, path::PathBuf};

#[derive(Clone)]
pub struct EventStore {
    directory: PathBuf,
}

impl EventStore {
    pub fn new(directory: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create event directory: {error}"))?;
        Ok(Self { directory })
    }

    pub fn save(&self, event: &SlowdownEvent) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(event)
            .map_err(|error| format!("failed to serialize event: {error}"))?;
        let path = self.path(&event.id)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|error| format!("failed to write event: {error}"))?;
        fs::rename(&temporary, &path).map_err(|error| format!("failed to commit event: {error}"))
    }

    pub fn get(&self, id: &str) -> Result<SlowdownEvent, String> {
        let bytes =
            fs::read(self.path(id)?).map_err(|error| format!("failed to read event: {error}"))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("failed to parse stored event: {error}"))
    }

    pub fn list(&self) -> Result<Vec<EventSummary>, String> {
        let mut summaries = Vec::new();
        for entry in fs::read_dir(&self.directory)
            .map_err(|error| format!("failed to list events: {error}"))?
        {
            let entry = entry.map_err(|error| format!("failed to read event entry: {error}"))?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let bytes = match fs::read(entry.path()) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            let event: SlowdownEvent = match serde_json::from_slice(&bytes) {
                Ok(event) => event,
                Err(_) => continue,
            };
            summaries.push(EventSummary {
                id: event.id,
                clicked_at_ns: event.clicked_at_ns,
                status: event.status,
                severity: event
                    .jev_result
                    .as_ref()
                    .map(|result| result.slowdown_severity.score),
                primary_process: event.jev_result.as_ref().map(|result| {
                    result
                        .processes
                        .iter()
                        .find(|process| process.process_id == result.primary_cause.process_id)
                        .map(|process| process.name.clone())
                        .unwrap_or_else(|| "system / unknown".into())
                }),
            });
        }
        summaries.sort_by_key(|event| std::cmp::Reverse(event.clicked_at_ns));
        Ok(summaries)
    }

    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err("invalid event id".into());
        }
        Ok(self.directory.join(format!("{id}.json")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SlowdownStatus;
    use std::collections::BTreeMap;

    fn event() -> SlowdownEvent {
        SlowdownEvent {
            id: "123e4567-e89b-12d3-a456-426614174000".into(),
            clicked_at_ns: 2,
            window_start_ns: 1,
            window_end_ns: 3,
            status: SlowdownStatus::Complete,
            system_samples: vec![],
            processes: vec![],
            capabilities: BTreeMap::new(),
            jev_request: None,
            jev_result: None,
            errors: vec![],
        }
    }

    #[test]
    fn round_trips_event_and_lists_summary() {
        let directory = tempfile::tempdir().unwrap();
        let store = EventStore::new(directory.path().to_owned()).unwrap();
        store.save(&event()).unwrap();
        assert_eq!(store.get(&event().id).unwrap().clicked_at_ns, 2);
        assert_eq!(store.list().unwrap().len(), 1);
    }
}
