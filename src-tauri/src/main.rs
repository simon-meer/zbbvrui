// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::Ipv4Addr;
use std::process::Command;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::scrcpy::ScrCpy;
use adb_client::AdbTcpConnection;
use log::{info, log};
use tauri_plugin_log::LogTarget;

use crate::structs::{LocalDevice, ZBBError};

mod scrcpy;
mod structs;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_devices() -> Result<Vec<LocalDevice>, ZBBError> {
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;

    let result = adb
        .devices()?
        .into_iter()
        .map(|it| it.into())
        .collect::<Vec<_>>();

    Ok(result)
}

#[tauri::command]
async fn connect_device(id: String, port: u16, app_handle: AppHandle) -> Result<String, ZBBError> {
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;

    let serial = Some(id.clone());

    let result = adb.shell_command(
        &serial,
        vec![
            "getprop".to_string(),
            "service.adb.tcp.port".to_string(),
        ],
    )?;

    let configured_port = String::from_utf8(result).unwrap();

    info!("Configured port: {}", &configured_port);
    if configured_port.trim() != port.to_string() {
        // Switch to TCP
        let adb_exe = app_handle
            .path_resolver()
            .resolve_resource("scrcpy/adb.exe")
            .ok_or(ZBBError::ADB("ADB nicht gefunden".to_string()))?;

        let result = Command::new(adb_exe)
            .args(vec![
                "-s".to_string(),
                id.clone(),
                "tcpip".to_string(),
                port.to_string(),
            ])
            .output()?;

        info!(
            "tcpip result: {}",
            String::from_utf8(result.stdout).unwrap()
        );

        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(1000));

            if get_devices()?.iter().any(|it| &it.identifier == &id) {
                break;
            }
        }
    }

    let ip_address: Ipv4Addr = get_ip(id.clone())?
        .parse()
        .map_err(|err| ZBBError::Other(format!("{:?}", err)))?;

    if let Err(result) = adb.connect(ip_address, port) {
        if !result.to_string().contains(" already connected ") {
            return Err(result.into());
        }
    }

    Ok(ip_address.to_string())
}

#[tauri::command]
fn get_ip(id: String) -> Result<String, ZBBError> {
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;
    let serial = Some(id);

    let ip_route = adb.shell_command(&serial, vec!["ip".to_string(), "route".to_string()])?;
    let ip_address: String = String::from_utf8(ip_route)
        .expect("Failed to parse `ip route` result")
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|line| line.len() >= 9)
        .nth(0)
        .map(|line| line[8])
        .ok_or(ZBBError::Other(
            "Konnte die IP Addresse nicht finden.".to_string(),
        ))?
        .to_string();

    Ok(ip_address)
}

#[tauri::command]
async fn open_stream(id: String, app_handle: AppHandle) -> Result<(), ZBBError> {
    ScrCpy::open_window(&id, &app_handle)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_devices,
            connect_device,
            get_ip,
            open_stream
        ])
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .setup(|app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
