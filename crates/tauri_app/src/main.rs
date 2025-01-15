// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;
use log::info;
use system_shutdown::shutdown;
use tauri::{Manager, State};
use tauri_plugin_log::LogTarget;

use window_manager::WindowError;

use crate::adb::*;
use crate::communication::{get_phase, set_phase};
use crate::structs::*;
use crate::util::*;

mod adb;
mod structs;
mod util;
mod communication;

#[tauri::command]
async fn get_window_position(pid: u32) -> Result<window_manager::Position, WindowError> {
    window_manager::get_window_position(pid)
}

#[tauri::command]
async fn set_window_position(
    pid: u32,
    position: window_manager::Position,
) -> Result<(), WindowError> {
    window_manager::set_window_position(pid, position)
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_adb_path(paths: State<Paths>) -> Result<String, ZBBError> {
    paths.adb.clone().ok_or(ZBBError::ADB("ADB nicht gefunden".to_string()))
}

#[tauri::command]
fn get_scrcpy_path(paths: State<Paths>) -> Result<String, ZBBError> {
    paths.scrcpy.clone().ok_or(ZBBError::ADB("ADB nicht gefunden".to_string()))
}


#[tauri::command]
fn shutdown_host() -> Result<(), ZBBError> {
    #[cfg(not(dev))]
    shutdown()?;

    #[cfg(dev)]
    std::thread::sleep(Duration::from_millis(1000));

    Ok(())
}

fn main() {
    let _ = fix_path_env::fix(); // <---- Add this

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_devices,
            connect_device,
            connect_to_ip,
            get_ip,
            get_adb_path,
            get_scrcpy_path,
            get_window_position,
            set_window_position,
            is_running,
            launch_app,
            shutdown_device,
            get_battery_level,
            is_screen_on,
            kill_server,
            kill_app,
            shutdown_host,
            get_phase,
            set_phase
        ])
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .setup(|app| {
            let paths = Paths::new(
                find_binary("adb", app.handle(), true),
                find_binary("scrcpy", app.handle(), !is_windows()),
            );
            let res = app.manage(paths);
            info!("{}", res);
            info!("{:?}", app.state::<Paths>());

            launch_adb(app.state());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
