import {
    Component,
    computed,
    CUSTOM_ELEMENTS_SCHEMA,
    effect,
    ElementRef,
    input,
    OnDestroy,
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
import '@sbb-esta/lyne-elements/status.js';

import {DeviceService} from "../device.service";
import {
    defer,
    distinctUntilChanged,
    filter,
    finalize, firstValueFrom,
    from,
    interval,
    map,
    retry,
    Subscription, switchMap,
    tap,
    timer
} from "rxjs";
import {Device, DeviceState} from "../../domain/device.model";
import {takeUntilDestroyed} from "@angular/core/rxjs-interop";
import {SbbStepperElement} from "@sbb-esta/lyne-elements/stepper.js";
import {NgIf} from "@angular/common";
import {ScrcpyService} from "../scrcpy.service";
import {Child} from "@tauri-apps/api/shell";
import {SettingsService} from "../settings.service";
import {Position} from "../../domain/position.model";
import {ZBBError} from "../../domain/zbberror.model";
import {NotificationService} from "../notification.service";

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
export class DeviceComponent implements OnInit, OnDestroy {
    public name = input.required<string>();
    public id = input.required<string>();
    public port = input.required<number>();
    public ip = signal<string | undefined>(undefined);
    public localDevice = signal<Device | undefined>(undefined);
    public remoteDevice = signal<Device | undefined>(undefined);
    public isMirroring = signal(false);
    public mirroringActivated = signal(false);
    public enforceAppActivated = signal(false);
    public scrcpyProcess = signal<undefined | Child>(undefined);
    public lastPosition = signal<Position | undefined>(undefined);
    public batteryLevel = signal<number | undefined>(undefined);
    public batteryLevelIcon = computed(() => {
        const batteryLevel = this.batteryLevel();
        if(batteryLevel === undefined) {
            return '';
        }
        if (batteryLevel > 75) {
            return 'battery-level-full-small';
        }
        if (batteryLevel > 50) {
            return 'battery-level-medium-small';
        }
        if (batteryLevel > 15) {
            return 'battery-level-low-small';
        }

        return 'battery-level-empty-small';
    });

    protected connectionError = signal<string | undefined>(undefined);
    public isBusy = false;

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
        private _settingsService: SettingsService,
        private _notificationService: NotificationService
    ) {
        this._deviceService.observeDevices().pipe(
            takeUntilDestroyed(),
            filter(_ => !this.isBusy),
            map(devices => this.extractDevices(devices)),
            distinctUntilChanged((lhs, rhs) => JSON.stringify(lhs) === JSON.stringify(rhs))
        ).subscribe(device => this.onDeviceChanged(device[0], device[1]));

        // Connect when state allows for it
        effect((onCleanup) => {
            const state = this.state();

            this._stepper.nativeElement.selectedIndex = state === State.WaitingForDevice || state === State.Authorizing
                ? 0
                : state === State.WaitingForRemoteConnection
                    ? 1
                    : 2;

            if (state === State.WaitingForRemoteConnection) {
                const subscription = this.startConnecting(this.id(), this.port());
                onCleanup(() => {
                    subscription.unsubscribe();
                });
            }

        });

        // Start mirroring when toggle changes or we are connected
        effect(() => {
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

        // Keep settings in sync
        effect(() => {
            if (this._syncingSettings) return;

            const settings = this._settingsService.getSettings(this.id());

            settings.ip = this.ip();
            settings.keepAppRunning = this.enforceAppActivated();
            settings.keepMirroring = this.mirroringActivated();
            settings.lastWindowPosition = this.lastPosition();

            console.log('update settings', settings);

            this._settingsService.setSettings(settings);
        });


        // Keep app running
        effect((onCleanup) => {
            if (this.state() !== State.Ready || !this.enforceAppActivated()) {
                return;
            }

            // Ticks since last success
            let handle: number | undefined = undefined;
            let retryCount = 0;
            const check = async () => {
                try {
                    if (await this._deviceService.isScreenOn(this.ip()!) === true &&
                        !await this._deviceService.isRunning(this.ip()!, this._settingsService.getPackageName())) {
                        console.log('launch')
                        
                        await this._deviceService.launch(this.ip()!, this._settingsService.getPackageName());
                        retryCount++;
                    } else {
                        retryCount = 0;
                    }
                } finally {
                    handle = setTimeout(check.bind(this), 1000 * Math.pow(2, retryCount));
                }
            }

            handle = setTimeout(check.bind(this), 1000);

            onCleanup(() => {
                clearTimeout(handle);
            });
        });

        // Update battery
        effect((onCleanup) => {
            if(this.state() !== State.Ready) {
                return;
            }

            const subscription = timer(0, 30_000).pipe(
                switchMap(_ => from(this._deviceService.getBatteryLevel(this.ip()!)))
            ).subscribe(batteryLevel => {
                this.batteryLevel.set(batteryLevel);
            });

            onCleanup(subscription.unsubscribe);
        }, {
            allowSignalWrites: true
        });
    }

    ngOnDestroy(): void {
        this.scrcpyProcess()?.kill();
    }

    ngOnInit(): void {
        this.applySettings();
    }

    private startConnecting(id: string, port: number): Subscription {
        // Try connecting until the subscription has been canceled or the connection has been established
        return defer(() => {
            console.log('start connecting', this.isBusy);

            this.isBusy = true;
            return from(this._deviceService.connect(id, port));
        }).pipe(
            tap({
                next: () => this.isBusy = false,
                error: (e) => {
                    this.isBusy = false;
                    this.handleError(e);
                }
            }),
            retry({
                delay: 1000
            })
        ).subscribe((ip) => {
            console.log('connection established');

            this.ip.set(ip);
            this.connectionError.set(undefined);
        });
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

    private handleError(e: ZBBError) {
        console.error(e);

        switch (e.type) {
            case "NotInANetwork":
                this.connectionError.set('Die Brille scheint nicht mit dem Netzwerk verbunden zu sein. Bitte überprüfe die WLAN-Einstellungen der Brille und stelle sicher, dass der Router eingesteckt ist.');
                break;
            case "NotInSameNetwork":
                this.connectionError.set('Die Brille scheint nicht im gleichen Netzwerk wie der PC zu sein. Stelle sicher, dass der PC mit dem Router verbunden ist.');
                break;
            case "ADB":
                this._notificationService.showToast(`Fehler beim Ausführen von ADB: ${e.message}`);
                break;
            case "IO":
                this._notificationService.showToast(`Interner Fehler: ${e.message}`);
                break;
            case "Other":
                this._notificationService.showToast(`Interner Fehler: ${e.message}`);
                break;
        }
    }

    protected readonly State = State;
}
