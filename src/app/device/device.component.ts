import {
    Component,
    computed,
    CUSTOM_ELEMENTS_SCHEMA,
    effect,
    ElementRef,
    input,
    OnInit,
    signal,
    ViewChild
} from '@angular/core';

import '@sbb-esta/lyne-elements/card.js';
import '@sbb-esta/lyne-elements/title.js';
import '@sbb-esta/lyne-elements/stepper.js';
import '@sbb-esta/lyne-elements/loading-indicator.js';
import '@sbb-esta/lyne-elements/toggle-check.js';
import '@sbb-esta/lyne-elements/notification.js';
import {DeviceService} from "../device.service";
import {distinctUntilChanged, filter, finalize, lastValueFrom, map, takeLast, tap} from "rxjs";
import {Device, DeviceState} from "../../domain/device.model";
import {takeUntilDestroyed} from "@angular/core/rxjs-interop";
import {SbbStepperElement} from "@sbb-esta/lyne-elements/stepper.js";
import {NgIf} from "@angular/common";
import {ScrcpyService} from "../scrcpy.service";
import {Child} from "@tauri-apps/api/shell";
import {SettingsService} from "../settings.service";
import {Position} from "../../domain/position.model";

enum State {
    WaitingForDevice,
    Authorizing,
    WaitingForRemoteConnection,
    Ready
}

@Component({
    selector: 'app-device',
    standalone: true,
    imports: [
        NgIf
    ],
    schemas: [CUSTOM_ELEMENTS_SCHEMA],
    templateUrl: './device.component.html',
    styleUrl: './device.component.css'
})
export class DeviceComponent implements OnInit {
    public name = input.required<string>();
    public id = input.required<string>();
    public port = input.required<number>();
    public ip = signal<string | undefined>(undefined);
    public localDevice = signal<Device | undefined>(undefined);
    public remoteDevice = signal<Device | undefined>(undefined);
    public isMirroring = signal(false);
    public isBusy = signal(false);
    public mirroringActivated = signal(false);
    public enforceAppActivated = signal(false);
    public scrcpyProcess = signal<undefined | Child>(undefined);
    public lastPosition = signal<Position | undefined>(undefined);

    private _syncingSettings = true;

    public state = computed(() => {
        const remote = this.remoteDevice();
        const local = this.localDevice();

        if (remote && remote.state == DeviceState.Device) {
            return State.Ready;
        }

        switch (local?.state) {
            case undefined:
            case DeviceState.NoDevice:
            case DeviceState.Offline:
                return State.WaitingForDevice;

            case DeviceState.Authorizing:
            case DeviceState.Unauthorized:
                return State.Authorizing;

            case DeviceState.Device:
                return State.WaitingForRemoteConnection;
        }
    });

    @ViewChild('stepper')
    private _stepper!: ElementRef<SbbStepperElement>;

    constructor(
        private _deviceService: DeviceService,
        private _scrcpyService: ScrcpyService,
        private _settingsService: SettingsService
    ) {
        this._deviceService.observeDevices().pipe(
            takeUntilDestroyed(),
            filter(_ => !this.isBusy()),
            map(devices => this.extractDevices(devices)),
            distinctUntilChanged((lhs, rhs) => JSON.stringify(lhs) === JSON.stringify(rhs))
        ).subscribe(device => this.onDeviceChanged(device[0], device[1]));

        effect(() => {
            const state = this.state();

            this._stepper.nativeElement.selectedIndex = state === State.WaitingForDevice || state === State.Authorizing
                ? 0
                : state === State.WaitingForRemoteConnection
                    ? 1
                    : 2;

            if (state === State.WaitingForRemoteConnection) {
                this.isBusy.set(true);
                try {
                    this._deviceService.connect(this.id()!, this.port()).then(ip => {
                        this.ip.set(ip);
                    });
                } finally {
                    this.isBusy.set(false);
                }
            }
        }, {
            allowSignalWrites: true
        });

        effect(() => {
            console.log("OK", this.mirroringActivated(), this.state());
            if (this.mirroringActivated() && this.state() == State.Ready) {
                this.startMirror();
            } else if (this.isMirroring()) {
                this.scrcpyProcess()?.kill().catch(e => {
                    console.error(e);
                });
            }
        }, {
            allowSignalWrites: true
        });

        effect(() => {
            if (this._syncingSettings) return;
            console.log('update settings');

            const settings = this._settingsService.getSettings(this.id());

            settings.ip = this.ip();
            settings.keepAppRunning = this.enforceAppActivated();
            settings.keepMirroring = this.mirroringActivated();
            settings.lastWindowPosition = this.lastPosition();

            console.log('update settings', settings);

            this._settingsService.setSettings(settings);
        });
    }

    ngOnInit(): void {
        this.applySettings();
    }

    private applySettings() {
        this._syncingSettings = true;
        try {
            const settings = this._settingsService.getSettings(this.id());

            this.ip.set(settings.ip);
            this.mirroringActivated.set(settings.keepMirroring);
            this.enforceAppActivated.set(settings.keepAppRunning);
            this.lastPosition.set(settings.lastWindowPosition);

            console.log('loaded settings', settings);
        } finally {
            this._syncingSettings = false;
        }
    }

    private extractDevices(devices: Device[]): [Device | undefined, Device | undefined] {
        return [
            devices.find(it => it.identifier === this.id()),
            devices.find(it => it.identifier === `${this.ip()}:${this.port()}`)
        ];
    }

    private onDeviceChanged(localDevice: Device | undefined, remoteDevice: Device | undefined) {
        console.log(localDevice, remoteDevice);
        this.remoteDevice.set(remoteDevice);
        this.localDevice.set(localDevice);
    }

    async startMirror() {
        if (this.isMirroring()) {
            return;
        }

        this.isMirroring.set(true);
        this._scrcpyService.spawnScrcpy(this.ip()!, this.lastPosition()).pipe(
            finalize(() => {
                this.isMirroring.set(false);
                this.scrcpyProcess.set(undefined);
            })
        ).subscribe((e) => {
            switch (e.type) {
                case "process":
                    console.log("set prorcess", e.process);
                    this.scrcpyProcess.set(e.process);
                    break;
                case "stream":
                    if (e.pipe === 'stdout') {
                        console.log(e.content);
                    } else {
                        console.error(e.content);
                    }
                    break;
                case "window":
                    console.log(e.position);
                    this.lastPosition.set(e.position);
                    break;

            }
        });
    }

    protected readonly State = State;
}
