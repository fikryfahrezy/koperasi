mod application;
mod commands;
mod contracts;
mod database;
mod domain;
mod runtime;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    runtime::run();
}
