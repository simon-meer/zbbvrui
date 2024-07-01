import {Injectable} from '@angular/core';
import {invoke} from "@tauri-apps/api/tauri";
import {Child, Command} from "@tauri-apps/api/shell";
import {from, map, mergeWith, Observable, ReplaySubject, retry, startWith, switchMap, takeUntil, timer} from "rxjs";
import {Position} from "../domain/position.model";
import {SettingsService} from "./settings.service";
import {os} from "@tauri-apps/api";

type CommandEvent = StreamEvent | ProcessEvent | WindowEvent;

interface StreamEvent {
    type: 'stream',
    pipe: 'stdout' | 'stderr',
    content: string
}

interface ProcessEvent {
    type: 'process',
    process: Child
}

interface WindowEvent {
    type: 'window',
    position: Position,
    process: Child,
}


@Injectable({
    providedIn: 'root'
})
export class ScrcpyService {

    constructor(private settingsService: SettingsService) {

    }

    async getAdbPath() {
        return invoke<string>('get_adb_path');
    }

    async getScrcpyPath() {
        return invoke<string>('get_scrcpy_path');
    }

    async setPosition(pid: number, position: Position) {
        await invoke("set_window_position", {
            pid,
            position
        });
    }

    spawnScrcpy(id: string, position?: Position): Observable<CommandEvent> {
        const args = ['-s', id, ...this.settingsService.getScrcpyArguments().split(/\s+/g)];

        if (position) {
            args.push('--window-x', position.x + '');
            args.push('--window-y', position.y + '');
            args.push('--window-width', position.width + '');
            args.push('--window-height', position.height + '');
        }

        const subject: ReplaySubject<CommandEvent> = new ReplaySubject(5);
        const destroy: ReplaySubject<void> = new ReplaySubject(1);

        return from(os.platform()).pipe(
            map(platform => {
                const isWindows = platform === 'win32';
                const cmd = new Command(isWindows ? "srcpy_embedded" : "scrcpy", args);

                cmd.stdout.on('data', line => subject.next({
                    type: 'stream',
                    pipe: 'stdout',
                    content: line
                }));

                cmd.stderr.on('data', line => subject.next({
                    type: 'stream',
                    pipe: 'stderr',
                    content: line
                }));

                cmd.on('close', (data) => {
                    console.log(`command finished with code ${data.code} and signal ${data.signal}`)
                    subject.complete();
                    destroy.next();
                });

                cmd.on('error', (e) => {
                    subject.error(e);
                    destroy.next();
                });

                return cmd;
            }),
            switchMap(cmd => cmd.spawn()),
            switchMap((child) => {
                return subject.pipe(
                    mergeWith(
                        timer(1000, 1000).pipe(
                            switchMap(() =>
                                invoke<Position>('get_window_position', {
                                    pid: child.pid
                                })),
                            map(position => {
                                return {
                                    type: 'window',
                                    position,
                                    process: child
                                } as WindowEvent
                            }),
                            retry(),
                            takeUntil(destroy)
                        )
                    ),
                    startWith({
                        type: 'process',
                        process: child,
                    } as ProcessEvent)
                );
            })
        )
    }
}
