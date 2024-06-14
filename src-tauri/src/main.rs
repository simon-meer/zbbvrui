// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::OsStr;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::Duration;

use log::{info, log, warn};
use network_interface::{NetworkInterface};
use network_interface::NetworkInterfaceConfig;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_log::LogTarget;
use which::which;

use adb_client::{AdbTcpConnection, RustADBError};
use window_manager::WindowError;

use crate::scrcpy::ScrCpy;
use crate::structs::{LocalDevice, Paths, ZBBError};

mod scrcpy;
mod structs;

const LOOPBACK: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const ADB_PORT: u16 = 5037;

#[cfg(target_os = "windows")]
fn create_silent_command<S>(path: S) -> Command where S: AsRef<OsStr> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    Command::new(path).creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(target_os = "windows"))]
fn create_silent_command<S>(path: S) -> Command where S: AsRef<OsStr> + Debug {
    Command::new(path)
}

fn launch_adb(paths: State<Paths>) {
    create_silent_command(paths.adb.as_ref().expect("ADB not found"))
        .args(vec!["devices".to_string()])
        .output()
        .expect("Unable to start ADB");
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

fn find_binary(name: &str, handle: AppHandle, search_path: bool) -> Option<String> {
    info!("Looking for {}", name);
    if search_path && which(name).is_ok() {
        info!("Found it installed");
        return Some(name.to_string());
    }

    info!("Using the packaged binaries");
    handle
        .path_resolver()
        .resolve_resource("scrcpy/scrcpy.exe")
        .and_then(|buf| buf.to_str().map(|s| s.to_string()))
}

#[tauri::command]
async fn get_devices<'a, 'b>(paths: State<'a, Paths>) -> Result<Vec<LocalDevice>, ZBBError> {
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT).or_else(|error| {
        if let RustADBError::IOError(io_error) = error {
            warn!("Unable to connect to adb: {:?}", io_error);

            // Launch ADB
            launch_adb(paths);

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
async fn connect_device<'a>(id: String, port: u16, paths: State<'a, Paths>) -> Result<String, ZBBError> {
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let serial = Some(id.clone());

    let result = adb.shell_command(
        &serial,
        vec!["getprop".to_string(), "service.adb.tcp.port".to_string()],
    )?;

    let configured_port = String::from_utf8(result).unwrap();

    info!("Configured port: {}", &configured_port);
    if configured_port.trim() != port.to_string() {
        // Switch to TCP

        let adb = paths.adb.as_ref().unwrap();
        let result = create_silent_command(adb)
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
            async_std::task::sleep(Duration::from_millis(1000)).await;

            if get_devices(paths.clone())
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
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;
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
                return is_match(ip, other, netmask);
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
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let result = adb.shell_command(&serial, vec!["pidof".into(), package])?;

    Ok(!result.is_empty())
}


#[tauri::command]
async fn is_screen_on(id: String) -> Result<bool, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let result = adb.shell_command(&serial, vec!["dumpsys deviceidle | grep mScreenOn".into()])?;
    let result_string = String::from_utf8(result).map_err(|err| ZBBError::Other(err.to_string()))?;

    Ok(result_string.split_once('=').map(|it| it.1.trim() == "true").unwrap_or(false))
}


#[tauri::command]
async fn launch_app(id: String, package: String) -> Result<String, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    // Disable proximity sensor to get the device out of sleep
    // If we don't do this, the device sometimes gets into a weird state
    let _ = adb.shell_command(&serial, vec!["am broadcast -a com.oculus.vrpowermanager.prox_close".to_string()]);

    let bytes = adb.shell_command(
        &serial,
        vec!["monkey".into(), "-p".into(), package, "1".into()],
    )?;

    // Enable proximity sensor again
    let _ = adb.shell_command(&serial, vec!["am broadcast -a com.oculus.vrpowermanager.automation_disable".to_string()]);

    let result = String::from_utf8(bytes).unwrap();
    Ok(result)
}

#[tauri::command]
async fn shutdown_device(id: String) -> Result<(), ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    adb.shell_command(&serial, vec!["reboot".into(), "-p".into()])?;

    Ok(())
}

#[tauri::command]
async fn get_battery_level(id: String) -> Result<i32, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let level_bytes = adb.shell_command(&serial, vec!["dumpsys battery | grep level".into()])?;
    let level_string = String::from_utf8(level_bytes).map_err(|it| ZBBError::Other("Konnte Batteriestand nicht holen.".into()))?;

    let level = level_string.split(':').into_iter().nth(1).and_then(|number| number.trim().parse::<i32>().ok());

    match level {
        Some(battery) => Ok(battery),
        None => Err(ZBBError::Other("Unbekannter Batteriestand".into()))
    }
}


#[cfg(target_os = "windows")]
fn is_windows() -> bool { true }

#[cfg(not(target_os = "windows"))]
fn is_windows() -> bool { false }


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
            shutdown_device,
            get_battery_level,
            is_screen_on
        ])
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .build(),
        )
        .setup(|app| {
            let paths = Paths::new(
                find_binary("adb", app.handle(), true),
                find_binary("scrcpy", app.handle(), !is_windows())
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_check() {
        let ip1 = Ipv4Addr::new(192, 168, 1, 5);
        let ip2 = Ipv4Addr::new(192, 168, 1, 155);
        let ip3 = Ipv4Addr::new(192, 168, 2, 155);
        let netmask = Ipv4Addr::new(255, 255, 255, 0);
        let netmask2 = Ipv4Addr::new(255, 255, 0, 0);

        assert_eq!(true, is_match(ip1, ip2, netmask));
        assert_eq!(false, is_match(ip1, ip3, netmask));
        assert_eq!(true, is_match(ip1, ip3, netmask2));
    }
}