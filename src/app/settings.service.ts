import {Injectable} from '@angular/core';
import {DeviceSettings} from "../domain/device-settings.model";

@Injectable({
    providedIn: 'root'
})
export class SettingsService {

    constructor() {
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
        localStorage.setItem(settings.id, JSON.stringify(settings));
    }
}
