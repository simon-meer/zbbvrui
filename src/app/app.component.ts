import {Component, CUSTOM_ELEMENTS_SCHEMA, signal} from '@angular/core';
import {CommonModule} from '@angular/common';
import {RouterOutlet} from '@angular/router';

import '@sbb-esta/lyne-elements/button.js';
import "@sbb-esta/lyne-elements/dialog.js";
import "@sbb-esta/lyne-elements/overlay.js";
import "@sbb-esta/lyne-elements/title.js";
import "@sbb-esta/lyne-elements/toast.js";

import {DeviceComponent} from "./device/device.component";
import {SettingsComponent} from "./settings/settings.component";
import {SettingsService} from "./settings.service";
import {SbbDialogElement} from "@sbb-esta/lyne-elements/dialog.js";

@Component({
    selector: 'app-root',
    standalone: true,
    schemas: [CUSTOM_ELEMENTS_SCHEMA],
    imports: [CommonModule, RouterOutlet, DeviceComponent, SettingsComponent],
    templateUrl: './app.component.html',
    styleUrl: './app.component.css',
})
export class AppComponent {
    packageName = signal(this._settingsService.getPackageName());
    scrcpyArguments = signal(this._settingsService.getScrcpyArguments());
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

    openOverlay(overlayEl: HTMLElement, e: MouseEvent) {
        const overlay = overlayEl as SbbDialogElement;
        overlay.open();
    }

    onDialogClosed(e: Event) {
        console.log(e);
        this._settingsService.setPackageName(this.packageName());
        this._settingsService.setDeviceSerials(this.devices());
        this._settingsService.setScrcpyArguments(this.scrcpyArguments());
    }
}
