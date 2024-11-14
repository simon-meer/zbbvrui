import {Injectable} from '@angular/core';
import {DeviceSettings} from "../domain/device-settings.model";

@Injectable({
    providedIn: 'root'
})
export class SettingsService {

    constructor() {
    }

    getPackageName(): string {
        return localStorage.getItem('package') ?? 'ch.sbb.xr.zbbvr';
    }

    getCleanPackageName(): string {
        const packageName = this.getPackageName();
        const slashPos = packageName.indexOf("/");
        if(slashPos >= 0) {
            return packageName.substring(0, slashPos);
        }

        return packageName;
    }

    setPackageName(activity: string) {
        localStorage.setItem('package', activity);
    }

    getDeviceSerials(): string[] {
        return JSON.parse(localStorage.getItem('devices') ?? '[]');
    }

    setDeviceSerials(devices: string[]) {
        localStorage.setItem('devices', JSON.stringify(devices));
    }

    getSettings(id: string) {
        const jsonString = localStorage.getItem(`device.${id}`);

        if (!jsonString) {
            console.log('creating new settings');
        }

        return jsonString
            ? JSON.parse(jsonString) as DeviceSettings
            : {
                id,
                activityName: 'ch.sbb.xr.zbbvr',
                keepAppRunning: false,
                keepMirroring: false
            };
    }

    setSettings(settings: DeviceSettings) {
        localStorage.setItem(`device.${settings.id}`, JSON.stringify(settings));
    }

    getScrcpyArguments(): string {
        return localStorage.getItem('scrcpyArgs') ??  '--crop=2064:2208:2064:100 --rotation-offset=-22 --scale=195 --position-x-offset=-520 --position-y-offset=-490 --video-bit-rate=16M --max-size 1080';
    }

    setScrcpyArguments(args: string) {
        localStorage.setItem('scrcpyArgs', args);
    }
}
