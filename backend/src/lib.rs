//! Raid Scheduler backend library.
//!
//! The binary (`main.rs`) only wires configuration, logging, HTTP serving and
//! the Telegram bot. All domain logic lives here so it can be tested and reused
//! from both the bot and the Mini App API.

pub mod api;
pub mod config;
pub mod db;
pub mod domain;
pub mod services;
