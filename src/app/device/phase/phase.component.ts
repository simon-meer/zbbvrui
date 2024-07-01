import {Component, CUSTOM_ELEMENTS_SCHEMA, effect, ElementRef, input, signal, ViewChild} from '@angular/core';
import {PhaseService} from "../../phase.service";
import {takeUntilDestroyed, toObservable} from "@angular/core/rxjs-interop";
import {filter, map, switchMap, tap} from "rxjs";

import '@sbb-esta/lyne-elements/radio-button.js';
import '@sbb-esta/lyne-elements/loading-indicator.js';
import '@sbb-esta/lyne-elements/toggle.js';

import {NotificationService} from "../../notification.service";
import {NgIf} from "@angular/common";
import {Phase} from "../../../domain/phase.model";
import {SbbToggleElement, SbbToggleOptionElement} from "@sbb-esta/lyne-elements/toggle.js";


@Component({
    selector: 'app-phase',
    standalone: true,
    imports: [
        NgIf
    ],
    schemas: [CUSTOM_ELEMENTS_SCHEMA],
    templateUrl: './phase.component.html',
    styleUrl: './phase.component.css'
})
export class PhaseComponent {
    ip = input.required<string>();
    phase = signal<string | undefined>('Onboarding');
    loading = signal(false);

    @ViewChild("toggle")
    toggle!: ElementRef<SbbToggleElement>;

    constructor(private phaseService: PhaseService, private notificationService: NotificationService) {
        toObservable(this.ip).pipe(
            takeUntilDestroyed(),
            switchMap(ip => this.phaseService.observeAppPhase(ip)),
            filter(_ => !this.loading()),
            map(it => (it === Phase.Windup) ? Phase.Onboarding : it)
        ).subscribe((state) => {
            this.phase.set(state);
        });

        effect(() => {
            const phase = this.phase();
            // Toggle is buggy, so we need to set it twice
            if (phase && this.toggle) {
                this.toggle.nativeElement.value = phase;
            }
        });
    }

    async onChange(targetPhaseString: string) {
        const targetPhase = targetPhaseString as Phase;
        console.log("SET", targetPhase, this.phase());
        if (!targetPhase || targetPhase == this.phase()) return;

        this.loading.set(true);
        try {
            await this.phaseService.setAppPhase(this.ip(), targetPhase);
            this.phase.set(targetPhase);

        } catch (e) {
            this.notificationService.showToast(`Konnte die Szene nicht wechseln: {e}`);
            this.phase.set(await this.phaseService.getAppPhase(this.ip()));
        } finally {
            this.loading.set(false);
        }

    }

    protected readonly Phase = Phase;
}
