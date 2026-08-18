import { LiteGraph, LGraphNode } from '@comfyorg/litegraph';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { logStore } from '$lib/stores/logStore.svelte';

export type BootMode = 'auto' | 'uefi' | 'bios' | 'disk';

export interface MachineConfig {
    id: string;
    iso_path: string | null;
    disk_path: string;
    ram_mb: number;
    cpus: number;
    shared_folder: string | null;
    boot_mode: BootMode;
    network: {
        lan: boolean;
        internet: boolean;
        port_forwards: { host: number; guest: number; proto: string }[];
    };
    input: { keyboard_passthrough: boolean };
}

export interface FrameSnapshot {
    w: number;
    h: number;
    data: string; // base64 RGBA
}

export class MachineNode extends LGraphNode {
    static override title = 'Machine';

    // Set by overlay manager
    overlayEl: HTMLDivElement | null = null;
    onSyncRect?: (x: number, y: number, w: number, h: number, visible: boolean) => void;
    onDestroy?: () => void;

    private _state: 'stopped' | 'starting' | 'running' | 'paused' | 'error' = 'stopped';
    lastError: string = '';
    private _unlisten: UnlistenFn[] = [];
    private _vmCanvas: HTMLCanvasElement | null = null;
    private _vmCtx: CanvasRenderingContext2D | null = null;
    private _thumbnail: ImageData | null = null;
    private _rafId: number | null = null;
    private _pollId: number | null = null;
    private _paintLogCount = 0;
    private _lastFrameW = 0;
    private _lastFrameH = 0;
    private _lastFrameImage: ImageData | null = null;
    // Reused per-frame buffer + ImageData to avoid GC churn at 30fps.
    private _frameBuf: Uint8ClampedArray | null = null;
    private _frameImgData: ImageData | null = null;
    private _stateListeners = new Set<() => void>();

    onStateChange(fn: () => void): () => void {
        this._stateListeners.add(fn);
        return () => { this._stateListeners.delete(fn); };
    }

    constructor() {
        super('Machine');
        this.title = 'Machine';
        this.size = [540, 360];

        // Graph I/O ports
        this.addInput('stdin',      'string');
        this.addOutput('stdout',    'string');
        this.addOutput('exit_code', 'number');
        this.addOutput('files',     'string');

        if (!this.properties.id) {
            this.properties.id = crypto.randomUUID();
        }
        this.properties = {
            id:            this.properties.id,
            iso_path:      '/tmp/nonos.iso',
            disk_path:     '',
            ram_mb:        2048,
            cpus:          2,
            shared_folder: undefined,
            boot_mode:     'auto',
            network: { lan: false, internet: false, port_forwards: [] },
            input:   { keyboard_passthrough: true },
            ...this.properties,
        };
    }

    get vmId(): string { return this.properties.id as string; }
    get state(): typeof this._state { return this._state; }
    get config(): MachineConfig { return this.properties as unknown as MachineConfig; }

    async start(): Promise<void> {
        if (this._state === 'running' || this._state === 'starting') return;
        this._setState('starting');
        try {
            await invoke('vm_start', { config: this.config });
            await this._startListening();
        } catch (e) {
            this.lastError = String(e);
            this._setState('error');
            logStore.error(`vm_start failed: ${String(e)}`);
        }
    }

    async stop(): Promise<void> {
        await invoke('vm_stop', { id: this.vmId }).catch(console.error);
        this._saveThumbnail();
        this._stopRaf();
        this._setState('stopped');
        this._stopListening();
    }

    async pause(): Promise<void> {
        await invoke('vm_pause', { id: this.vmId }).catch(console.error);
        this._setState('paused');
    }

    async resume(): Promise<void> {
        await invoke('vm_resume', { id: this.vmId }).catch(console.error);
        this._setState('running');
    }

    async sendClipboardToVm(text: string): Promise<void> {
        await invoke('vm_clipboard_to_vm', { id: this.vmId, text }).catch(console.error);
    }

    async sendInput(event: object): Promise<void> {
        if (this._state !== 'running') return;
        await invoke('vm_input', { id: this.vmId, event }).catch(() => {});
    }

    setVmCanvas(canvas: HTMLCanvasElement): void {
        this._vmCanvas = canvas;
        this._vmCtx    = canvas.getContext('2d');
        // Re-apply the latest frame so the new canvas isn't blank after a swap
        // (e.g. fullscreen toggle). Without this the frontend would have to wait
        // for the next dirty frame from the VM, which can be many seconds idle.
        if (this._lastFrameImage && this._vmCtx) {
            canvas.width  = this._lastFrameW;
            canvas.height = this._lastFrameH;
            this._vmCtx.putImageData(this._lastFrameImage, 0, 0);
        } else if (this._state === 'stopped' && this._thumbnail) {
            this._vmCtx?.putImageData(this._thumbnail, 0, 0);
        }
    }

    private async _startListening(): Promise<void> {
        const ul1 = await listen<{ vm_id: string; data: string }>('vm:stdout', (ev) => {
            if (ev.payload.vm_id !== this.vmId) return;
            this.setOutputData(0, ev.payload.data);
        });
        const ul2 = await listen<{ vm_id: string; text: string }>('vm:clipboard', (ev) => {
            if (ev.payload.vm_id !== this.vmId) return;
            navigator.clipboard.writeText(ev.payload.text).catch(() => {});
        });
        this._unlisten.push(ul1, ul2);
        this._startRaf();
    }

    private _startRaf(): void {
        // VNC reader pushes frames at ~30fps; polling at 60fps is wasted IPC.
        // Frame transport is raw binary — ArrayBuffer with [w:u32 LE | h:u32 LE | rgba ...].
        // Empty body means no new frame (skip the paint).
        let busy = false;
        let statusTick = 0;
        const tick = () => {
            if (this._state === 'stopped' || this._state === 'error') return;
            if (busy) return;
            busy = true;

            statusTick++;
            const framePromise  = invoke<ArrayBuffer>('vm_get_frame_bin', { id: this.vmId });
            const statusPromise = (statusTick % 30 === 0)
                ? invoke<{ state: string } | null>('vm_status', { id: this.vmId })
                : Promise.resolve(null);

            Promise.all([framePromise, statusPromise])
                .then(([buf, status]) => {
                    if (status?.state === 'running' && this._state === 'starting') {
                        this._setState('running');
                    }
                    if (buf && buf.byteLength >= 8) {
                        if (this._state === 'starting') this._setState('running');
                        this._rafId = requestAnimationFrame(() => this._applyBinaryFrame(buf));
                    }
                })
                .catch(() => {})
                .finally(() => { busy = false; });
        };
        this._pollId = setInterval(tick, 33) as unknown as number;
    }

    private _applyBinaryFrame(buf: ArrayBuffer): void {
        const ctx = this._vmCtx;
        if (!ctx || !this._vmCanvas) return;
        const view = new DataView(buf);
        const w = view.getUint32(0, true);
        const h = view.getUint32(4, true);
        const expected = w * h * 4;
        if (buf.byteLength < 8 + expected) return;
        // Resize canvas if framebuffer size changed.
        if (this._vmCanvas.width !== w || this._vmCanvas.height !== h) {
            this._vmCanvas.width  = w;
            this._vmCanvas.height = h;
            // Buffer geometry changed — drop reused buffer so we re-allocate.
            this._frameBuf     = null;
            this._frameImgData = null;
        }
        // Reuse a Uint8ClampedArray across frames to avoid GC churn.
        // Note: putImageData requires the typed array to back ImageData;
        // we copy into the reused buffer rather than allocate fresh each time.
        if (!this._frameBuf || this._frameBuf.length !== expected) {
            this._frameBuf     = new Uint8ClampedArray(expected);
            this._frameImgData = new ImageData(this._frameBuf, w, h);
        }
        this._frameBuf.set(new Uint8Array(buf, 8, expected));
        ctx.putImageData(this._frameImgData!, 0, 0);
        this._lastFrameW = w;
        this._lastFrameH = h;
        this._lastFrameImage = this._frameImgData;
    }

    private _stopRaf(): void {
        if (this._pollId !== null) { clearInterval(this._pollId); this._pollId = null; }
        if (this._rafId  !== null) { cancelAnimationFrame(this._rafId);  this._rafId  = null; }
    }

    private _stopListening(): void {
        this._unlisten.forEach(fn => fn());
        this._unlisten = [];
    }


    private _applySnapshot(snap: FrameSnapshot): void {
        const ctx = this._vmCtx;
        if (!ctx || !this._vmCanvas) return;
        if (this._vmCanvas.width !== snap.w || this._vmCanvas.height !== snap.h) {
            this._vmCanvas.width  = snap.w;
            this._vmCanvas.height = snap.h;
        }
        const bin = atob(snap.data);
        const buf = new Uint8ClampedArray(bin.length);
        for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
        const img = new ImageData(buf, snap.w, snap.h);
        ctx.putImageData(img, 0, 0);
        this._lastFrameImage = img;
        this._lastFrameW = snap.w;
        this._lastFrameH = snap.h;
    }

    private _saveThumbnail(): void {
        if (!this._vmCtx || !this._vmCanvas) return;
        try {
            this._thumbnail = this._vmCtx.getImageData(
                0, 0, this._vmCanvas.width, this._vmCanvas.height
            );
        } catch { /* tainted canvas */ }
    }

    private _setState(s: typeof this._state): void {
        if (this._state === s) return;
        this._state = s;
        (this as any).setDirtyCanvas?.(true, false);
        for (const fn of this._stateListeners) fn();
    }

    destroy(): void {
        this.stop().catch(() => {});
        this._stopRaf();
        this._stopListening();
        this.onDestroy?.();
    }
}

export function registerMachineNode(): void {
    if ((LiteGraph as any).registered_node_types?.['knofoo/machine']) return;
    (MachineNode as any).title = 'Machine';
    (MachineNode as any).desc  = 'Virtual machine — boots any ISO, embedded in graph';
    LiteGraph.registerNodeType('knofoo/machine', MachineNode);
}
