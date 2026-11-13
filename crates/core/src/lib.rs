//! Aircraft Monitor core: geometry, N-number conversion, providers, engine, notifications,
//! persistence and the HTTP API used by both the headless server and the desktop shell.

pub mod airports;
pub mod api;
pub mod app;
pub mod engine;
pub mod geo;
pub mod model;
pub mod nnumber;
pub mod notify;
pub mod providers;
pub mod secrets;
pub mod settings;
pub mod store;
pub mod ui;
