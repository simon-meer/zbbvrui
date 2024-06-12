// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::{IpAddr, Ipv4Addr};
use std::ops::Index;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use log::{error, info, log, warn};
use network_interface::{Netmask, NetworkInterface};
use network_interface::NetworkInterfaceConfig;
use serde::Serialize;
use tauri::api::Error::Utf8;
use tauri::{AppHandle, Position};
use tauri_plugin_log::LogTarget;

use adb_client::{AdbTcpConnection, RustADBError};
use window_manager::WindowError;

use crate::scrcpy::ScrCpy;
use crate::structs::{LocalDevice, ZBBError};

mod scrcpy;
mod structs;

const LOOPBACK: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const ADB_PORT: u16 = 5037;

fn launch_adb(app_handle: AppHandle) {
    let adb_exe = app_handle
        .path_resolver()
        .resolve_resource("scrcpy/adb.exe")
        .ok_or(ZBBError::ADB("ADB nicht gefunden".to_string()))
        .expect("ADB not found");

    Command::new(adb_exe)
        .args(vec!["devices".to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .expect("Unable to start ADB");
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_adb_path(app_handle: AppHandle) -> Result<PathBuf, ZBBError> {
    app_handle
        .path_resolver()
        .resolve_resource("scrcpy/adb.exe")
        .ok_or(ZBBError::ADB("ADB nicht gefunden".to_string()))
}

#[tauri::command]
fn get_scrcpy_path(app_handle: AppHandle) -> Result<PathBuf, ZBBError> {
    app_handle
        .path_resolver()
        .resolve_resource("scrcpy/scrcpy.exe")
        .ok_or(ZBBError::ADB("ADB nicht gefunden".to_string()))
}

#[tauri::command]
async fn get_devices(app_handle: AppHandle) -> Result<Vec<LocalDevice>, ZBBError> {
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT).or_else(|error| {
        if let RustADBError::IOError(io_error) = error {
            warn!("Unable to connect to adb: {:?}", io_error);

            // Launch ADB
            launch_adb(app_handle);

            return AdbTcpConnection::new(LOOPBACK, ADB_PORT);
        }

        Err(error)
    })?;

    let result = adb
        .devices()?
        .into_iter()
        .map(|it| it.into())
        .collect::<Vec<_>>();

    Ok(result)
}

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

#[tauri::command]
async fn connect_device(id: String, port: u16, app_handle: AppHandle) -> Result<String, ZBBError> {
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;

    let serial = Some(id.clone());

    let result = adb.shell_command(
        &serial,
        vec!["getprop".to_string(), "service.adb.tcp.port".to_string()],
    )?;

    let configured_port = String::from_utf8(result).unwrap();

    info!("Configured port: {}", &configured_port);
    if configured_port.trim() != port.to_string() {
        // Switch to TCP
        let adb_exe = &app_handle
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
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        info!(
            "tcpip result: {}",
            String::from_utf8(result.stdout).unwrap()
        );

        for _ in 0..5 {
            async_std::task::sleep(Duration::from_millis(1000)).await;

            if get_devices(app_handle.clone())
                .await?
                .iter()
                .any(|it| &it.identifier == &id)
            {
                break;
            }
        }
    }

    let ip_address: Ipv4Addr = get_ip(id.clone())?
        .parse()
        .map_err(|err| ZBBError::Other(format!("{:?}", err)))?;

    // Check if we're in the same network
    test_network(ip_address)?;

    if let Err(result) = adb.connect(ip_address, port) {
        if !result.to_string().contains(" already connected ") {
            return Err(result.into());
        }
    }

    Ok(ip_address.to_string())
}

/// Gets the IP address of a Android device
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
        .ok_or(ZBBError::NotInANetwork)?
        .to_string();

    Ok(ip_address)
}

/// Tests if [other] is in the same network as the host machine.
///
/// For simplicity's sake, we assume a netmask
fn test_network(other: Ipv4Addr) -> Result<(), ZBBError> {
    let network_interfaces =
        NetworkInterface::show().map_err(|it| ZBBError::Other(it.to_string()))?;

    let mut addresses = network_interfaces
        .into_iter()
        .flat_map(|interface| interface.addr);

    if !addresses.any(|addr| {
        if let IpAddr::V4(ip) = addr.ip() {
            if let Some(IpAddr::V4(netmask)) = addr.netmask() {
                return is_match(ip, other, netmask)
            }
        }

        false
    }) {
        return Err(ZBBError::NotInSameNetwork);
    }

    Ok(())
}

fn is_match(lhs: Ipv4Addr, rhs: Ipv4Addr, netmask: Ipv4Addr) -> bool {
    netmask.octets().into_iter().enumerate().all(|(pos, mask)| {
        lhs.octets()[pos] & mask == rhs.octets()[pos] & mask
    })
}

#[tauri::command]
async fn open_stream(id: String, app_handle: AppHandle) -> Result<(), ZBBError> {
    ScrCpy::open_window(&id, &app_handle)
}

#[tauri::command]
async fn is_running(id: String, package: String) -> Result<bool, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;

    let result = adb.shell_command(&serial, vec!["pidof".into(), package])?;

    Ok(!result.is_empty())
}

#[tauri::command]
async fn launch_app(id: String, package: String) -> Result<String, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;

    let bytes = adb.shell_command(
        &serial,
        vec!["monkey".into(), "-p".into(), package, "1".into()],
    )?;
    let result = String::from_utf8(bytes).unwrap();
    Ok(result)
}

#[tauri::command]
async fn shutdown_device(id: String) -> Result<(), ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(Ipv4Addr::from([127, 0, 0, 1]), 5037)?;

    adb.shell_command(&serial, vec!["reboot".into(), "-p".into()])?;

    Ok(())
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_devices,
            connect_device,
            get_ip,
            open_stream,
            get_adb_path,
            get_scrcpy_path,
            get_window_position,
            set_window_position,
            is_running,
            launch_app,
            shutdown_device
        ])
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .setup(|app| {
            launch_adb(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_check() {
        let ip1 = Ipv4Addr::new(192, 168, 1, 5);
        let ip2 = Ipv4Addr::new(192, 168, 1, 155);
        let ip3 = Ipv4Addr::new(192, 168, 2, 155);
        let netmask = Ipv4Addr::new(255,255,255,0);
        let netmask2 = Ipv4Addr::new(255,255,0,0);

        assert_eq!(true, is_match(ip1, ip2, netmask));
        assert_eq!(false, is_match(ip1, ip3, netmask));
        assert_eq!(true, is_match(ip1, ip3, netmask2));
    }
}