import { Injectable } from '@angular/core';
import {Observable, share, shareReplay, switchMap, timer} from "rxjs";
import {invoke} from "@tauri-apps/api/tauri";
import {Device} from "../domain/device.model";
import {fromPromise} from "rxjs/internal/observable/innerFrom";

@Injectable({
  providedIn: 'root'
})
export class DeviceService {
  private devices$ = timer(1000, 500)
      .pipe(
          switchMap(() => {
            return fromPromise(invoke<Device[]>('get_devices'))
          }),
          share()
      )

  constructor() {}

  observeDevices() {
    return this.devices$;
  }

  async connect(id: string, port: number) {
      return await invoke<string>('connect_device', {
          id,
          port
      })
  }

  async getIp(id: string) {
      return invoke<string>('get_ip', {
          id
      });
  }

  async mirror(id: string): Promise<void> {
      return invoke('open_stream', {
          id
      });
  }

  async isRunning(id: string, packageName: string) {
      return invoke<boolean>('is_running', {
          id,
          'package': packageName
      })
  }


    async isScreenOn(id: string) {
        return invoke<boolean>('is_screen_on', {
            id
        }).catch(e => undefined);
    }

  async launch(id: string, packageName: string) {
      return invoke<boolean>('launch_app', {
          id,
          'package': packageName
      })
  }

  async getBatteryLevel(id: string) {
      return invoke<number>('get_battery_level', {
          id
      }).catch(e => {
          return undefined;
      });
  }
}
