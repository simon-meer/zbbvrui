export interface Device {
    /// Unique device identifier.
    identifier: String,
        /// Connection state of the device.
    state: DeviceState
}

export interface DeviceLong {
    /// Unique device identifier.
    identifier: String,
    /// Connection state of the device.
    state: DeviceState,
    /// Usb port used by the device.
    usb: String,
    /// Product code.
    product: String,
    /// Device model.
    model: String,
    /// Device code.
    device: String,
    /// Transport identifier.
    transport_id: number,
}


export enum DeviceState {
    /**
     * The device is not connected to adb or is not responding.
     */
    Offline= 'Offline',

    /**
     * The device is now connected to the adb server. Note that this state does not imply that the Android system is fully booted and operational because the device connects to adb while the system is still booting. However, after boot-up, this is the normal operational state of an device.
     */
    Device = 'Device',
    /// There is no device connected.
    NoDevice = 'NoDevice',
    /// Device is being authorized
    Authorizing = 'Authorizing',
    /// The device is unauthorized.
    Unauthorized = 'Unauthorized',
}