import {Position} from "./position.model";

export interface DeviceSettings {
    id: string,
    keepMirroring: boolean,
    keepAppRunning: boolean,
    activityName: string,

    ip?: string,
    lastWindowPosition?: Position,
}