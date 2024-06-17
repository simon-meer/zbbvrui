use std::io;
use std::io::Error;
use adb_client::{Device, DeviceState, RustADBError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LocalDeviceState {
    /// The device is not connected to adb or is not responding.
    Offline,
    /// The device is now connected to the adb server. Note that this state does not imply that the Android system is fully booted and operational because the device connects to adb while the system is still booting. However, after boot-up, this is the normal operational state of an device.
    Device,
    /// There is no device connected.
    NoDevice,
    /// Device is being authorized
    Authorizing,
    /// The device is unauthorized.
    Unauthorized,
}

impl From<DeviceState> for LocalDeviceState {
    fn from(state: DeviceState) -> Self {
        match state {
            DeviceState::Offline => LocalDeviceState::Offline,
            DeviceState::Device => LocalDeviceState::Device,
            DeviceState::NoDevice => LocalDeviceState::NoDevice,
            DeviceState::Authorizing => LocalDeviceState::Authorizing,
            DeviceState::Unauthorized => LocalDeviceState::Unauthorized
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalDeviceLong {
    /// Unique device identifier.
    pub identifier: String,
    /// Connection state of the device.
    pub state: LocalDeviceState,
    /// Usb port used by the device.
    pub usb: String,
    /// Product code.
    pub product: String,
    /// Device model.
    pub model: String,
    /// Device code.
    pub device: String,
    /// Transport identifier.
    pub transport_id: u32,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalDevice {
    /// Unique device identifier.
    pub identifier: String,
    /// Connection state of the device.
    pub state: LocalDeviceState,
}

impl From<Device> for LocalDevice {
    fn from(device: Device) -> Self {
        LocalDevice {
            identifier: device.identifier,
            state: device.state.into(),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Paths {
    pub adb: Option<String>,
    pub scrcpy: Option<String>
}

impl Paths {
    pub fn new(adb: Option<String>, scrcpy: Option<String>) -> Self {
        Self { adb, scrcpy }
    }
}


#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum ZBBError {
    ADB(String),
    IO(String),
    NotInANetwork,
    NotInSameNetwork,
    Other(String)
}

impl From<RustADBError> for ZBBError {
    fn from(value: RustADBError) -> Self {
        ZBBError::ADB(value.to_string())
    }
}

impl From<io::Error> for ZBBError {
    fn from(value: Error) -> Self {
        ZBBError::IO(value.to_string())
    }
}