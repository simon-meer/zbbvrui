import { Injectable } from '@angular/core';
import {SbbToastElement} from "@sbb-esta/lyne-elements/toast.js";

@Injectable({
  providedIn: 'root'
})
export class NotificationService {

  constructor() { }

  showToast(message: string) {
    const toast = document.getElementById('warning-toast') as SbbToastElement | undefined;

    console.log(toast);
    if(toast) {
      toast.close();
      toast.textContent = message;
      toast.open();
    }
  }
}
