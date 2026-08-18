import { LGraphNode } from '@comfyorg/litegraph';

const BACKEND    = 'http://127.0.0.1:7422';
const WS_BACKEND = 'ws://127.0.0.1:7422';

async function getRFB() {
    // Loaded from static/ at runtime — not bundled
    const url = '/novnc/core/rfb.js';
    const mod = await import(/* @vite-ignore */ url);
    return mod.default;
}

export class VncNode extends LGraphNode {
    static override title = 'Machine';

    private overlay:   HTMLDivElement | null = null;
    private vncTarget: HTMLDivElement | null = null;
    private rfb:       any = null;
    private sessionId: string | null = null;
    private status:    'idle' | 'booting' | 'ready' | 'error' = 'idle';

    constructor() {
        super('Machine', 'knofoo/vnc');
        this.size = [640, 420];
        this.addOutput('session', 'string');
    }

    mount(parentEl: HTMLElement) {
        this.overlay = document.createElement('div');
        this.overlay.style.cssText = `
            position:absolute; pointer-events:auto;
            background:#0d0d0d;
            border:1px solid rgba(255,255,255,0.1);
            border-radius:8px; overflow:hidden;
            display:flex; flex-direction:column;
            font-family:monospace;
        `;

        // Header
        const header = document.createElement('div');
        header.style.cssText = `
            display:flex; align-items:center; gap:8px;
            padding:6px 10px; height:34px; flex-shrink:0;
            background:rgba(255,255,255,0.04);
            border-bottom:1px solid rgba(255,255,255,0.07);
            box-sizing:border-box;
        `;
        header.innerHTML = `
            <span style="width:10px;height:10px;border-radius:50%;
                         background:#f472b6;flex-shrink:0"></span>
            <span style="font-size:11px;color:#e2e8f0;flex:1">
                NON-OS · v0.8.0-alpha
            </span>
            <span class="status" style="font-size:9px;color:rgba(255,255,255,0.3)">idle</span>
            <button class="boot-btn" style="
                padding:3px 12px;
                border:1px solid rgba(255,255,255,0.15);
                border-radius:4px;
                background:rgba(110,231,183,0.15);
                color:#6ee7b7; font-size:10px;
                font-family:monospace; cursor:pointer;
            ">Boot</button>
        `;
        this.overlay.appendChild(header);

        // VNC target — noVNC renders into this div
        this.vncTarget = document.createElement('div');
        this.vncTarget.style.cssText = `
            flex:1; min-height:0; background:#000;
            display:flex; align-items:center; justify-content:center;
        `;
        this.vncTarget.innerHTML = `
            <span style="color:rgba(255,255,255,0.2);font-size:12px;font-family:monospace">
                Click Boot to start
            </span>
        `;
        this.overlay.appendChild(this.vncTarget);

        parentEl.appendChild(this.overlay);

        header.querySelector('.boot-btn')!.addEventListener('click', (e) => {
            e.stopPropagation();
            this.boot();
        });
    }

    private async boot() {
        if (this.status === 'booting' || this.status === 'ready') return;
        this.setStatus('booting');
        if (this.vncTarget) {
            this.vncTarget.innerHTML = `
                <span style="color:#fbbf24;font-size:12px;font-family:monospace">
                    Starting VM...
                </span>
            `;
        }

        try {
            // Create session
            const res  = await fetch(`${BACKEND}/session`, { method: 'POST' });
            const data = await res.json();
            this.sessionId = data.session_id;

            // Wait for QEMU + websockify to be ready
            const ready = await fetch(`${BACKEND}/session/${this.sessionId}`);
            const info  = await ready.json();

            if (!info.ready) {
                throw new Error(info.error ?? 'VM failed to start');
            }

            // Connect noVNC directly through our aiohttp proxy
            this.vncTarget!.innerHTML = '';
            const RFB = await getRFB();
            this.rfb = new RFB(
                this.vncTarget!,
                `${WS_BACKEND}/vnc/${this.sessionId}`
            );
            this.rfb.scaleViewport = true;
            this.rfb.resizeSession = false;

            this.rfb.addEventListener('connect', () => {
                this.setStatus('ready');
                this.setOutputData(0, this.sessionId ?? '');
            });

            this.rfb.addEventListener('disconnect', (e: any) => {
                const reason = e.detail?.reason ?? 'unknown';
                console.error('noVNC disconnect:', reason, e.detail);
                this.setStatus('idle');
                if (this.vncTarget) {
                    this.vncTarget.innerHTML = `
                        <span style="color:#f472b6;font-size:11px;font-family:monospace;padding:8px;display:block">
                            Disconnected: ${reason}
                        </span>
                    `;
                }
            });

            this.rfb.addEventListener('securityfailure', (e: any) => {
                console.error('noVNC security failure:', e.detail);
            });

        } catch (err) {
            this.setStatus('error');
            if (this.vncTarget) {
                this.vncTarget.innerHTML = `
                    <span style="color:#f472b6;font-size:11px;font-family:monospace;padding:8px">
                        Error: ${err}
                    </span>
                `;
            }
        }
    }

    private setStatus(s: typeof this.status) {
        this.status = s;
        const el = this.overlay?.querySelector('.status') as HTMLElement;
        if (!el) return;
        const colors: Record<string, string> = {
            idle:    'rgba(255,255,255,0.3)',
            booting: '#fbbf24',
            ready:   '#6ee7b7',
            error:   '#f472b6',
        };
        el.style.color = colors[s];
        el.textContent = s;
    }

    syncRect(x: number, y: number, w: number, h: number, visible: boolean) {
        if (!this.overlay) return;
        const TITLE_H = 30;
        const oy = y + TITLE_H;
        const oh = h - TITLE_H;

        this.overlay.style.display = (visible && w > 40) ? 'flex' : 'none';
        this.overlay.style.left    = `${x}px`;
        this.overlay.style.top     = `${oy}px`;
        this.overlay.style.width   = `${w}px`;
        this.overlay.style.height  = `${oh}px`;
        this.overlay.style.opacity = w < 60 ? '0' : '1';

        // noVNC scales automatically via scaleViewport
    }

    destroy() {
        this.rfb?.disconnect();
        if (this.sessionId) {
            fetch(`${BACKEND}/session/${this.sessionId}`, { method: 'DELETE' }).catch(() => {});
        }
        this.overlay?.remove();
    }
}
