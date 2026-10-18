//! Transient provider text published token by token; never a durable transcript cursor.
//!
//! Text becomes publishable at each whitespace boundary, so the redactor sees
//! a whole token (and every earlier line) before any of it leaves the host.
//! Subscriptions send successive snapshots as ordered, offset-addressed
//! deltas (`subscription::live_text_deltas`).

use crate::backend::ProviderEvent;
use std::collections::BTreeMap;
use std::sync::Mutex;

pub const MAX_LIVE_TEXT_BYTES: usize = 4 * 1024;
const MAX_LIVE_PROMPTS: usize = crate::project_workspace::MAX_PROJECT_TABS;

#[derive(Default)]
struct Text {
    raw: String,
    complete: usize,
    withheld: bool,
    truncated: bool,
}

pub struct LiveTextSnapshot {
    pub text: String,
    pub truncated: bool,
}

#[derive(Default)]
pub struct LivePromptText(Mutex<BTreeMap<(String, String, String, String), Text>>);

#[derive(Default)]
pub struct LiveDesktopText(LivePromptText);

pub struct DesktopTextGuard<'a> {
    text: &'a LiveDesktopText,
    request: super::protocol::Envelope,
}

impl LiveDesktopText {
    pub fn begin(&self, project: &str, session: &str) -> DesktopTextGuard<'_> {
        let mut request = super::protocol::Envelope::new(
            super::protocol::Channel::Control,
            super::protocol::Message::Request {
                operation: super::protocol::Operation::Inspect,
                arguments: serde_json::json!({}),
            },
        );
        request.id = Some("desktop-preview".into());
        request.project_id = Some(project.into());
        request.session_id = Some(session.into());
        self.0.remove(&request, "desktop");
        DesktopTextGuard {
            text: self,
            request,
        }
    }
    pub fn projected(&self, project: &str, session: &str) -> Option<String> {
        self.0
            .snapshot("desktop", project, session, "desktop-preview")
            .filter(|value| !value.truncated)
            .map(|value| value.text)
    }
}

impl DesktopTextGuard<'_> {
    pub fn update(&self, event: &ProviderEvent) {
        self.text.0.update(&self.request, "desktop", event);
    }
}

impl Drop for DesktopTextGuard<'_> {
    fn drop(&mut self) {
        self.text.0.remove(&self.request, "desktop");
    }
}

impl LivePromptText {
    pub fn update(&self, request: &super::protocol::Envelope, device: &str, event: &ProviderEvent) {
        let (Some(id), Some(project), Some(session)) =
            (&request.id, &request.project_id, &request.session_id)
        else {
            return;
        };
        if matches!(event, ProviderEvent::ResponseCreated { .. }) {
            self.remove(request, device);
            return;
        }
        let (text, replace) = match event {
            ProviderEvent::TextDelta { delta } => (delta.as_str(), false),
            ProviderEvent::TextDone { text } => (text.as_str(), true),
            _ => return,
        };
        let mut values = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let key = (
            device.to_owned(),
            project.clone(),
            session.clone(),
            id.clone(),
        );
        if !values.contains_key(&key) && values.len() >= MAX_LIVE_PROMPTS {
            return;
        }
        let value = values.entry(key).or_default();
        if value.withheld || value.truncated {
            return;
        }
        if text.contains('\0') {
            value.raw.clear();
            value.complete = 0;
            value.withheld = true;
            super::wakeup::notify(session);
            return;
        }
        if text.len() > MAX_LIVE_TEXT_BYTES
            || (!replace && value.raw.len() + text.len() > MAX_LIVE_TEXT_BYTES)
        {
            // Keep only the previously publishable prefix, never slice an
            // overflowing incoming line before its redactor can see it.
            value.raw.truncate(value.complete);
            value.truncated = true;
            super::wakeup::notify(session);
            return;
        }
        if replace {
            value.raw.clear();
        }
        value.raw.push_str(text);
        // Publish per token, but never a partial credential-bearing word
        // before its redactor can see it whole.
        value.complete = if replace {
            value.raw.len()
        } else {
            value
                .raw
                .char_indices()
                .rev()
                .find(|(_, character)| character.is_whitespace())
                .map_or(0, |(at, character)| at + character.len_utf8())
        };
        super::wakeup::notify(session);
    }

    pub fn projected(
        &self,
        device: &str,
        project: &str,
        session: &str,
        id: &str,
    ) -> Option<String> {
        self.snapshot(device, project, session, id)
            .map(|value| value.text)
    }

    pub fn snapshot(
        &self,
        device: &str,
        project: &str,
        session: &str,
        id: &str,
    ) -> Option<LiveTextSnapshot> {
        let values = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let value = values.get(&(device.into(), project.into(), session.into(), id.into()))?;
        if value.withheld || (value.complete == 0 && !value.truncated) {
            return None;
        }
        let text = crate::security::redact_text(&value.raw[..value.complete], &[]);
        (text.len() <= MAX_LIVE_TEXT_BYTES).then_some(LiveTextSnapshot {
            text,
            truncated: value.truncated,
        })
    }

    pub fn remove(&self, request: &super::protocol::Envelope, device: &str) {
        if let (Some(id), Some(project), Some(session)) =
            (&request.id, &request.project_id, &request.session_id)
        {
            self.0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&(device.into(), project.clone(), session.clone(), id.clone()));
            super::wakeup::notify(session);
        }
    }
}
