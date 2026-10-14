//! Cross-model history transform (ZE-227, pi `transformMessages`).
//!
//! A turn's `native_history` annotation pins the provider, wire and model
//! that produced it. When the next request targets anything else, that turn
//! is replayed from its canonical text and tool calls instead: opaque
//! reasoning and signatures are dropped, visible thinking becomes plain text,
//! and tool-call IDs are rewritten into the target wire's limits. Turns that
//! belong to the target are never touched, so same-model replay stays
//! byte-identical (the transform returns `None` when nothing changes).
use std::collections::{BTreeMap, BTreeSet};

use crate::backend::OpenAiWireApi;
use crate::core::{Turn, TurnRole};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Where the next request goes.
#[derive(Clone, Copy)]
pub(crate) struct Target<'a> {
    pub wire: OpenAiWireApi,
    pub provider: &'a str,
    pub model: &'a str,
}

impl Target<'_> {
    /// Longest tool-call ID the wire accepts. Every wire gets the
    /// `[A-Za-z0-9_-]` alphabet Anthropic enforces.
    fn max_id_len(self) -> usize {
        match self.wire {
            // OpenAI Chat rejects `tool_calls[].id` longer than 40.
            OpenAiWireApi::ChatCompletions => 40,
            _ => 64,
        }
    }

    /// Whether `id` must be rewritten. A foreign turn gets the full limits;
    /// a turn with no native envelope has an unknown origin, so only an ID
    /// the target wire would reject outright (Anthropic) is rewritten.
    fn rewrites(self, id: &str, foreign: bool) -> bool {
        if foreign {
            !valid_id(id, self.max_id_len())
        } else {
            self.wire == OpenAiWireApi::AnthropicMessages && !valid_id(id, 64)
        }
    }

    fn owns(self, native: &Value) -> bool {
        native["wire"] == self.wire.as_str()
            && native["provider"] == self.provider
            && native.get("requested_model").unwrap_or(&native["model"]) == self.model
    }
}

fn annotations(turn: &Turn) -> &[Value] {
    turn.metadata
        .as_ref()
        .and_then(|metadata| metadata.get("annotations"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A turn produced by another provider, wire or model. An envelope that does
/// not name its origin is not attributable and is left for the encoder to
/// reject as before.
fn foreign(turn: &Turn, target: Target<'_>) -> bool {
    turn.role == TurnRole::Assistant
        && annotations(turn).iter().any(|a| {
            a["type"] == "native_history"
                && a["wire"].is_string()
                && a["provider"].is_string()
                && a.get("requested_model").unwrap_or(&a["model"]).is_string()
                && !target.owns(a)
        })
}

/// Visible thinking a foreign turn recorded. Redacted or signature-only
/// reasoning has no text and is dropped with its envelope.
fn visible_thinking(turn: &Turn) -> Vec<String> {
    let mut texts = Vec::new();
    for a in annotations(turn) {
        if a["type"] == "reasoning" {
            texts.extend(a["text"].as_str().map(str::to_owned));
        }
        if a["type"] != "native_history" {
            continue;
        }
        for block in a["blocks"].as_array().into_iter().flatten() {
            if block["type"] == "thinking" {
                texts.extend(block["thinking"].as_str().map(str::to_owned));
            }
        }
        for part in a["parts"].as_array().into_iter().flatten() {
            if part["thought"] == true {
                texts.extend(part["text"].as_str().map(str::to_owned));
            }
        }
    }
    texts.retain(|text| !text.trim().is_empty());
    texts
}

fn tool_calls_mut(turn: &mut Turn) -> Option<&mut Vec<Value>> {
    turn.metadata
        .as_mut()
        .and_then(|metadata| metadata.get_mut("tool_calls"))
        .and_then(Value::as_array_mut)
}

fn valid_id(id: &str, max: usize) -> bool {
    !id.is_empty()
        && id.len() <= max
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// Map `id` into the alphabet and length, keeping it unique in `taken`.
fn normalize_id(id: &str, max: usize, taken: &mut BTreeSet<String>) -> String {
    let mut base: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if base.is_empty() {
        base.push_str("call");
    }
    base.truncate(max);
    if taken.insert(base.clone()) {
        return base;
    }
    let digest = format!("{:x}", Sha256::digest(id.as_bytes()));
    let mut counter = 0_u64;
    loop {
        let suffix = format!("_{}{counter}", &digest[..8]);
        let mut candidate = base.clone();
        candidate.truncate(max.saturating_sub(suffix.len()));
        candidate.push_str(&suffix);
        if taken.insert(candidate.clone()) {
            return candidate;
        }
        counter += 1;
    }
}

/// Rewrite foreign turns (and tool-call IDs the target cannot accept) for
/// `target`; `None` when nothing changes, so the request is sent exactly as
/// recorded.
pub(crate) fn transform(turns: &[Turn], target: Target<'_>) -> Option<Vec<Turn>> {
    let rewrites_ids = |turn: &Turn| {
        let foreign = foreign(turn, target);
        turn.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("tool_calls"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|call| call["id"].as_str())
            .any(|id| target.rewrites(id, foreign))
    };
    if !turns.iter().any(|turn| {
        foreign(turn, target) || (turn.role == TurnRole::Assistant && rewrites_ids(turn))
    }) {
        return None;
    }
    let max = target.max_id_len();
    let mut taken: BTreeSet<String> = turns
        .iter()
        .filter_map(|turn| turn.metadata.as_ref()?.get("tool_calls")?.as_array())
        .flatten()
        .filter_map(|call| call["id"].as_str().map(str::to_owned))
        .collect();
    let mut renamed: BTreeMap<String, String> = BTreeMap::new();
    let mut out = turns.to_vec();
    for turn in &mut out {
        if turn.role != TurnRole::Assistant {
            continue;
        }
        let is_foreign = foreign(turn, target);
        for call in tool_calls_mut(turn).into_iter().flatten() {
            let Some(id) = call["id"].as_str().map(str::to_owned) else {
                continue;
            };
            if !target.rewrites(&id, is_foreign) {
                continue;
            }
            taken.remove(&id);
            let new = normalize_id(&id, max, &mut taken);
            call["id"] = Value::String(new.clone());
            renamed.insert(id, new);
        }
        if !is_foreign {
            continue;
        }
        let thinking = visible_thinking(turn);
        if !thinking.is_empty() {
            let mut text = thinking.join("\n\n");
            if !turn.content.is_empty() {
                text.push_str("\n\n");
                text.push_str(&turn.content);
            }
            turn.content = text;
        }
        if let Some(list) = turn
            .metadata
            .as_mut()
            .and_then(|metadata| metadata.get_mut("annotations"))
            .and_then(Value::as_array_mut)
        {
            list.retain(|a| a["type"] != "native_history" && a["type"] != "reasoning");
        }
    }
    if !renamed.is_empty() {
        for turn in &mut out {
            if turn.role != TurnRole::Tool {
                continue;
            }
            let Some(slot) = turn
                .metadata
                .as_mut()
                .and_then(|metadata| metadata.get_mut("tool_call_id"))
            else {
                continue;
            };
            if let Some(new) = slot.as_str().and_then(|id| renamed.get(id)) {
                *slot = Value::String(new.clone());
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn target(wire: OpenAiWireApi, provider: &'static str, model: &'static str) -> Target<'static> {
        Target {
            wire,
            provider,
            model,
        }
    }

    fn claude_turn(id: &str) -> Turn {
        let mut turn = Turn::new("a", TurnRole::Assistant, "answer");
        turn.metadata = Some(json!({
            "tool_calls": [{"id":id,"name":"read_file","arguments":{"path":"a"}}],
            "annotations": [
                {"type":"native_history","wire":"anthropic_messages","provider":"anthropic",
                 "model":"claude-sonnet-4-6-20260101","requested_model":"claude-sonnet-4-6",
                 "blocks":[{"type":"thinking","thinking":"plan it","signature":"sig"},
                           {"type":"redacted_thinking","data":"opaque"},
                           {"type":"text","text":"answer"},
                           {"type":"tool_use","id":id,"name":"read_file","input":{"path":"a"}}]},
                {"type":"anthropic_stop_reason","stop_reason":"tool_use"}
            ]
        }));
        turn
    }

    fn tool_turn(id: &str) -> Turn {
        let mut turn = Turn::new("t", TurnRole::Tool, "result");
        turn.metadata = Some(json!({"tool_call_id": id, "model_text": "result"}));
        turn
    }

    #[test]
    fn same_model_history_is_untouched() {
        let turns = [claude_turn("toolu_01"), tool_turn("toolu_01")];
        let same = target(
            OpenAiWireApi::AnthropicMessages,
            "anthropic",
            "claude-sonnet-4-6",
        );
        assert!(transform(&turns, same).is_none());
    }

    #[test]
    fn foreign_thinking_becomes_text_and_ids_fit_the_target() {
        let turns = [claude_turn("toolu.1:x"), tool_turn("toolu.1:x")];
        let out = transform(
            &turns,
            target(OpenAiWireApi::Responses, "openai", "gpt-5.4"),
        )
        .unwrap();
        assert_eq!(out[0].content, "plan it\n\nanswer");
        let metadata = out[0].metadata.as_ref().unwrap();
        assert_eq!(
            metadata["annotations"],
            json!([{"type":"anthropic_stop_reason","stop_reason":"tool_use"}])
        );
        assert_eq!(metadata["tool_calls"][0]["id"], "toolu_1_x");
        assert_eq!(
            out[1].metadata.as_ref().unwrap()["tool_call_id"],
            "toolu_1_x"
        );
        // Another Claude model is foreign too: the signature belongs to Sonnet.
        let opus = transform(
            &turns,
            target(
                OpenAiWireApi::AnthropicMessages,
                "anthropic",
                "claude-opus-4-6",
            ),
        )
        .unwrap();
        assert_eq!(opus[0].content, "plan it\n\nanswer");
    }

    #[test]
    fn normalized_ids_stay_unique_and_bounded() {
        let mut taken = BTreeSet::from(["a_b".to_owned()]);
        let id = normalize_id("a.b", 64, &mut taken);
        assert_ne!(id, "a_b");
        assert!(valid_id(&id, 64));
        let long = "x".repeat(100);
        let id = normalize_id(&long, 40, &mut taken);
        assert_eq!(id.len(), 40);
        let again = normalize_id(&format!("{long}y"), 40, &mut taken);
        assert!(valid_id(&again, 40) && again != id);
    }
}
