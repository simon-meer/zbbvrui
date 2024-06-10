import {Component, computed, CUSTOM_ELEMENTS_SCHEMA, effect, ElementRef, input, signal, ViewChild} from '@angular/core';

import '@sbb-esta/lyne-elements/card.js';
import '@sbb-esta/lyne-elements/title.js';
import '@sbb-esta/lyne-elements/stepper.js';
import '@sbb-esta/lyne-elements/loading-indicator.js';
import '@sbb-esta/lyne-elements/toggle-check.js';
import '@sbb-esta/lyne-elements/notification.js';
import {DeviceService} from "../device.service";
import {distinctUntilChanged, filter, map} from "rxjs";
import {Device, DeviceState} from "../../domain/device.model";
import {takeUntilDestroyed} from "@angular/core/rxjs-interop";
import {SbbStepperElement} from "@sbb-esta/lyne-elements/stepper.js";
import {NgIf} from "@angular/common";

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
export class DeviceComponent {
    public name = input.required<string>();
    public id = input.required<string>();
    public port = input.required<number>();
    public ip = signal<string | undefined>(undefined);
    public localDevice = signal<Device | undefined>(undefined);
    public remoteDevice = signal<Device | undefined>(undefined);
    public isMirroring = signal(false);
    public isBusy = signal(false);

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


    constructor(private _deviceService: DeviceService) {
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
        try {
            await this._deviceService.mirror(this.ip()!);
        } finally {
            this.isMirroring.set(false);
        }
    }

    protected readonly State = State;
}
