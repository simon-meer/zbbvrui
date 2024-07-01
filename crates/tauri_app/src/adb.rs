use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use adb_client::{AdbTcpConnection, RustADBError};
use log::{info, warn};
use tauri::State;
use crate::structs::{LocalDevice, Paths, ZBBError};
use crate::util::create_silent_command;


const LOOPBACK: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const ADB_PORT: u16 = 5037;

// #####################
// # ADB MANAGEMENT>   # 
// #####################

pub fn launch_adb(paths: State<Paths>) {
    create_silent_command(paths.adb.as_ref().expect("ADB not found"))
        .args(vec!["devices".to_string()])
        .output()
        .expect("Unable to start ADB");
}

#[tauri::command]
pub async fn kill_server() -> Result<(), ZBBError> {
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;
    adb.kill()?;

    Ok(())
}


// #####################
// # MANAGE CONNECTION # 
// #####################

#[tauri::command]
pub async fn get_devices<'a, 'b>(paths: State<'a, Paths>) -> Result<Vec<LocalDevice>, ZBBError> {
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
pub async fn launch_app(id: String, package: String) -> Result<String, ZBBError> {
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
pub async fn connect_device<'a>(id: String, port: u16, paths: State<'a, Paths>) -> Result<String, ZBBError> {
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
    crate::test_network(ip_address)?;

    if let Err(result) = adb.connect(ip_address, port) {
        if !result.to_string().contains(" already connected ") {
            return Err(result.into());
        }
    }

    Ok(ip_address.to_string())
}

#[tauri::command]
pub async fn connect_to_ip(ip_address: String, port: u16) -> Result<(), ZBBError> {
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;
    let address = Ipv4Addr::from_str(&ip_address)
        .map_err(|_| ZBBError::Other(format!("Invalid ip address: {}", ip_address)))?;

    adb.connect(address, port)?;

    Ok(())
}


#[tauri::command]
pub async fn kill_app(id: String, package: String) -> Result<(), ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;
    adb.shell_command(&serial, vec!["am".to_string(), "force-stop".to_string(), package])?;

    Ok(())
}

#[tauri::command]
pub async fn shutdown_device(id: String) -> Result<(), ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    adb.shell_command(&serial, vec!["reboot".into(), "-p".into()])?;

    Ok(())
}


// #####################
// # GET INFORMATION   # 
// #####################

#[tauri::command]
pub async fn is_running(id: String, package: String) -> Result<bool, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let result = adb.shell_command(&serial, vec!["pidof".into(), package])?;

    Ok(!result.is_empty())
}


#[tauri::command]
pub async fn is_screen_on(id: String) -> Result<bool, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let result = adb.shell_command(&serial, vec!["dumpsys deviceidle | grep mScreenOn".into()])?;
    let result_string = String::from_utf8(result).map_err(|err| ZBBError::Other(err.to_string()))?;

    Ok(result_string.split_once('=').map(|it| it.1.trim() == "true").unwrap_or(false))
}


#[tauri::command]
pub async fn get_battery_level(id: String) -> Result<i32, ZBBError> {
    let serial = Some(id);
    let mut adb = AdbTcpConnection::new(LOOPBACK, ADB_PORT)?;

    let level_bytes = adb.shell_command(&serial, vec!["dumpsys battery | grep level".into()])?;
    let level_string = String::from_utf8(level_bytes).map_err(|_| ZBBError::Other("Konnte Batteriestand nicht holen.".into()))?;

    let level = level_string.split(':').into_iter().nth(1).and_then(|number| number.trim().parse::<i32>().ok());

    match level {
        Some(battery) => Ok(battery),
        None => Err(ZBBError::Other("Unbekannter Batteriestand".into()))
    }
}


/// Gets the IP address of an Android device
#[tauri::command]
pub fn get_ip(id: String) -> Result<String, ZBBError> {
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
