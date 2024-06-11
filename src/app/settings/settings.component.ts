import {Component, CUSTOM_ELEMENTS_SCHEMA, OnDestroy, signal} from '@angular/core';
import {SettingsService} from "../settings.service";

import '@sbb-esta/lyne-elements/form-field.js';

@Component({
    selector: 'app-settings',
    standalone: true,
    imports: [],
    schemas: [CUSTOM_ELEMENTS_SCHEMA],
    templateUrl: './settings.component.html',
    styleUrl: './settings.component.css'
})
export class SettingsComponent implements OnDestroy {
    packageName = signal(this._settingsService.getPackageName());
    devices = signal<string[]>([]);

    constructor(private _settingsService: SettingsService) {
        const devices = _settingsService.getDeviceSerials();
        while (devices.length < 2) {
            devices.push("");
        }

        this.devices.set(devices);
    }

    ngOnDestroy(): void {
        console.log('save');
    }

    trackByIndex(index: number) {
        return index;
    }

    setDeviceSerial(i: number, value: string) {
        const devices = [...this.devices()];
        devices[i] = value;

        this.devices.set(devices);
    }
}
