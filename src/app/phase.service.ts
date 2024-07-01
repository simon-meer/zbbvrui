import {Injectable} from '@angular/core';
import {invoke} from "@tauri-apps/api/tauri";
import {defer, distinctUntilChanged, Observable, repeat} from "rxjs";
import {Phase} from '../domain/phase.model';

@Injectable({
    providedIn: 'root'
})
export class PhaseService {
    async setAppPhase(ip: string, phase: Phase) {
        return invoke<void>("set_phase", {
            ip,
            phase
        })
    }

    async getAppPhase(ip: string): Promise<Phase> {
        return invoke<Phase>('get_phase', {
            ip
        });
    }

    observeAppPhase(ip: string): Observable<Phase | undefined> {
        return defer(async () => {
            try {
                return await this.getAppPhase(ip);
            } catch (e) {
                console.error(e);
                return undefined;
            }
        }).pipe(
            repeat({
                delay: 10_000
            })
        );
    }
}
