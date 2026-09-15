mod application;
mod commands;
mod contracts;
mod database;
mod domain;
mod importer;
mod runtime;
mod state;
mod workbook;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    runtime::run();
}
