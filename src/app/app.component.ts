import {Component, CUSTOM_ELEMENTS_SCHEMA} from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterOutlet } from '@angular/router';
import { invoke } from "@tauri-apps/api/tauri";

import '@sbb-esta/lyne-elements/button.js';
import {DeviceComponent} from "./device/device.component";
import {Device} from "../domain/device.model";

@Component({
  selector: 'app-root',
  standalone: true,
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  imports: [CommonModule, RouterOutlet, DeviceComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css',
})
export class AppComponent {
  greetingMessage = "";

  greet(): void {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    invoke<Device[]>("get_devices").then((devices) => {
      console.log(devices);
    });
  }
}
