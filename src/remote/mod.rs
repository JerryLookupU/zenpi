//! Transport-independent Android/desktop wire primitives.
//!
//! This module does not bind a listener or grant remote execution authority.

pub mod approval;
pub mod auth;
pub mod command;
pub mod diff;
pub mod events;
pub mod files;
pub mod frame;
pub mod headless_host;
pub mod host_adapter;
pub mod lan;
pub mod listener;
pub mod live_text;
pub mod model;
pub mod pairing;
pub mod protocol;
pub mod resources;
pub mod sessions;
mod strict_json;
pub mod subscription;
pub mod terminal;
pub mod transcript;
pub mod wakeup;
pub mod workspace_api;
