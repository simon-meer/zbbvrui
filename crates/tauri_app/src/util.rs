use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use log::info;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use tauri::AppHandle;
use which::which;
use crate::structs::ZBBError;

#[cfg(target_os = "windows")]
pub fn create_silent_command<S>(path: S) -> Command where S: AsRef<OsStr> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = Command::new(path);
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd
}

#[cfg(not(target_os = "windows"))]
pub fn create_silent_command<S>(path: S) -> Command where S: AsRef<OsStr> {
    Command::new(path)
}


pub fn find_binary(name: &str, handle: AppHandle, search_path: bool) -> Option<String> {
    info!("Looking for {}", name);
    if search_path && which(name).is_ok() {
        info!("Found it installed");
        return Some(name.to_string());
    }

    let file_ending = if is_windows() {
        ".exe"
    } else {
        ""
    };

    info!("Using the packaged binaries");
    handle
        .path_resolver()
        .resolve_resource(format!("scrcpy/{}{}", name, file_ending))
        .and_then(|buf| buf.to_str().map(|s| s.to_string()))
}



#[cfg(target_os = "windows")]
pub fn is_windows() -> bool { true }

#[cfg(not(target_os = "windows"))]
pub fn is_windows() -> bool { false }



/// Tests if [other] is in the same network as the host machine.
///
/// For simplicity's sake, we assume a netmask
pub fn test_network(other: Ipv4Addr) -> Result<(), ZBBError> {
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

pub fn is_match(lhs: Ipv4Addr, rhs: Ipv4Addr, netmask: Ipv4Addr) -> bool {
    netmask.octets().into_iter().enumerate().all(|(pos, mask)| {
        lhs.octets()[pos] & mask == rhs.octets()[pos] & mask
    })
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