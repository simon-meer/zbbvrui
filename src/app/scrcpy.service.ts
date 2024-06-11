import {Injectable} from '@angular/core';
import {invoke} from "@tauri-apps/api/tauri";
import {Child, Command} from "@tauri-apps/api/shell";
import {
    from,
    map,
    mergeWith,
    Observable,
    onErrorResumeNext,
    ReplaySubject,
    retry,
    startWith,
    switchMap, takeUntil,
    timer
} from "rxjs";
import {Position} from "../domain/position.model";

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

    constructor() {

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
        const args = ['-s', id];

        if (position) {
            args.push('--window-x', position.x + '');
            args.push('--window-y', position.y + '');
            args.push('--window-width', position.width + '');
            args.push('--window-height', position.height + '');
        }

        const cmd = new Command("srcpy", args);
        const subject: ReplaySubject<CommandEvent> = new ReplaySubject(5);
        const destroy: ReplaySubject<void> = new ReplaySubject(1);

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

        cmd.on('close', () => {
            subject.complete();
            destroy.next();
        });

        cmd.on('error', () => {
            subject.complete();
            destroy.next();
        });

        return from(cmd.spawn()).pipe(
            switchMap((child) => {
                return subject.pipe(
                    mergeWith(
                        timer(1000, 1000).pipe(
                            switchMap(() => {
                                return from(invoke<Position>('get_window_position', {
                                    pid: child.pid
                                }))
                            }),
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
