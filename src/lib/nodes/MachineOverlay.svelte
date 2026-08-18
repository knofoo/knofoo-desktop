<script lang="ts">
    import { onMount, tick } from 'svelte';
    import type { MachineNode } from './MachineNode';
    import MachineConfig from './MachineConfig.svelte';
    import { logStore } from '$lib/stores/logStore.svelte';

    interface Props {
        node: MachineNode;
        onregister: (fn: (x: number, y: number, w: number, h: number, visible: boolean) => void) => void;
        onunregister: () => void;
    }

    let { node, onregister, onunregister }: Props = $props();

    const TITLE_H = 30;

    let overlayEl = $state<HTMLDivElement | undefined>(undefined);
    let canvasEl  = $state<HTMLCanvasElement | undefined>(undefined);
    let fsCanvasEl = $state<HTMLCanvasElement | undefined>(undefined);
    let fullscreen = $state(false);
    let showConfig = $state(false);
    let focused    = $state(false);
    let vmState    = $state('stopped');
    let vmError    = $state('');

    $effect(() => {
        vmState = node.state;
        vmError = node.lastError;
        const off = node.onStateChange(() => {
            vmState = node.state;
            vmError = node.lastError;
        });
        return off;
    });

    $effect(() => {
        if (canvasEl) node.setVmCanvas(canvasEl);
    });

    let lastRect: { x: number; y: number; w: number; h: number; visible: boolean } | null = null;
    $effect(() => {
        onregister((x, y, w, h, visible) => {
            if (!overlayEl) return;
            // Skip DOM writes when nothing actually moved — style writes
            // invalidate layout, even when the values are unchanged.
            if (lastRect
                && lastRect.x === x && lastRect.y === y
                && lastRect.w === w && lastRect.h === h
                && lastRect.visible === visible) return;
            lastRect = { x, y, w, h, visible };
            overlayEl.style.display = visible && w > 20 ? 'flex' : 'none';
            overlayEl.style.left    = `${x}px`;
            overlayEl.style.top     = `${y + TITLE_H}px`;
            overlayEl.style.width   = `${w}px`;
            overlayEl.style.height  = `${h - TITLE_H}px`;
        });
        return () => onunregister();
    });

    $effect(() => {
        function onKeyDown(e: KeyboardEvent) {
            if (!focused) return;
            if (!node.config.input.keyboard_passthrough) return;
            if (e.key === 'Escape' && fullscreen) { fullscreen = false; focused = false; return; }
            e.stopPropagation();
            node.sendInput({ type: 'key_down', code: e.code, key: e.key });
        }
        function onKeyUp(e: KeyboardEvent) {
            if (!focused) return;
            if (!node.config.input.keyboard_passthrough) return;
            e.stopPropagation();
            node.sendInput({ type: 'key_up', code: e.code, key: e.key });
        }
        window.addEventListener('keydown', onKeyDown);
        window.addEventListener('keyup', onKeyUp);
        return () => {
            window.removeEventListener('keydown', onKeyDown);
            window.removeEventListener('keyup', onKeyUp);
        };
    });

    function vmCoords(e: MouseEvent, el: HTMLElement) {
        // Map screen coords → fb pixel coords, accounting for object-fit:contain
        // letterboxing inside the wrap container. Use the currently-active canvas.
        const cv = fullscreen ? fsCanvasEl : canvasEl;
        const r = el.getBoundingClientRect();
        const cw = cv?.width  || r.width;
        const ch = cv?.height || r.height;
        const scale = Math.min(r.width / cw, r.height / ch);
        const renderedW = cw * scale;
        const renderedH = ch * scale;
        const offX = (r.width  - renderedW) / 2;
        const offY = (r.height - renderedH) / 2;
        const lx = e.clientX - r.left - offX;
        const ly = e.clientY - r.top  - offY;
        return { x: lx, y: ly, node_w: renderedW, node_h: renderedH };
    }
    let dragWrapEl: HTMLElement | null = null;

    function onPointerMove(e: PointerEvent) {
        if (vmState !== 'running') return;
        const wrap = dragWrapEl ?? (e.currentTarget as HTMLElement);
        node.sendInput({ type: 'mouse_move', ...vmCoords(e, wrap) });
    }
    function onPointerDown(e: PointerEvent) {
        focused = true;
        // Pull DOM focus into the overlay so window-level keydown handler
        // sees the overlay as event.target — graph delete shortcut skips
        // anything inside .machine-overlay/.fullscreen-modal.
        (e.currentTarget as HTMLElement)?.closest<HTMLElement>('.machine-overlay, .fullscreen-modal')?.focus();
        if (vmState !== 'running') return;
        const wrap = e.currentTarget as HTMLElement;
        dragWrapEl = wrap;
        e.preventDefault();
        node.sendInput({ type: 'mouse_down', button: e.button, ...vmCoords(e, wrap) });
        // Track drag at the window level so events keep flowing even when the
        // cursor leaves the wrap. Don't use setPointerCapture — it interferes
        // with the host cursor on some platforms.
        function onWinMove(ev: PointerEvent) {
            if (vmState !== 'running') return;
            node.sendInput({ type: 'mouse_move', ...vmCoords(ev, wrap) });
        }
        function onWinUp(ev: PointerEvent) {
            window.removeEventListener('pointermove', onWinMove);
            window.removeEventListener('pointerup', onWinUp);
            window.removeEventListener('pointercancel', onWinUp);
            dragWrapEl = null;
            if (vmState !== 'running') return;
            node.sendInput({ type: 'mouse_up', button: ev.button, ...vmCoords(ev, wrap) });
        }
        window.addEventListener('pointermove', onWinMove);
        window.addEventListener('pointerup', onWinUp);
        window.addEventListener('pointercancel', onWinUp);
    }
    function onPointerUp(_e: PointerEvent) {
        // Handled by the window listener registered in onPointerDown.
    }
    function onWheel(e: WheelEvent) {
        if (vmState !== 'running') return;
        e.preventDefault();
        node.sendInput({ type: 'wheel', delta_x: e.deltaX, delta_y: e.deltaY });
    }
    function onBlur() { focused = false; }

    async function syncClipboard() {
        try {
            const text = await navigator.clipboard.readText();
            await node.sendClipboardToVm(text);
        } catch { /* permission denied */ }
    }

    function resetSize() {
        const n = node as unknown as { size: [number, number]; setDirtyCanvas?: (a: boolean, b: boolean) => void };
        n.size = [540, 360];
        n.setDirtyCanvas?.(true, true);
    }

    async function toggleFullscreen() {
        fullscreen = !fullscreen;
        await tick();
        if (fullscreen ? fsCanvasEl : canvasEl) node.setVmCanvas((fullscreen ? fsCanvasEl : canvasEl)!);
        if (fullscreen) focused = true;
    }

    const stateColor: Record<string, string> = {
        stopped:  'rgba(255,255,255,0.25)',
        starting: '#fbbf24',
        running:  '#6ee7b7',
        paused:   '#93c5fd',
        error:    '#f472b6',
    };
</script>

<div class="machine-root" class:fs={fullscreen}>
    {#if !fullscreen}
    <div
        bind:this={overlayEl}
        class="machine-overlay"
        class:focused
        style:display="none"
        role="application"
        aria-label="Virtual machine display"
        onblur={onBlur}
        oncontextmenu={(e) => {
            e.preventDefault();
            const cv = document.querySelector('.lg-canvas, canvas') as HTMLCanvasElement | null;
            cv?.dispatchEvent(new MouseEvent('contextmenu', {
                clientX: e.clientX, clientY: e.clientY, bubbles: true, cancelable: true,
            }));
        }}
        tabindex="-1"
    >
        <div class="toolbar">
            <span class="state-dot" style:background={stateColor[vmState] ?? stateColor.stopped}></span>
            <span class="title-label">{node.title ?? 'Machine'}</span>
            <div class="spacer"></div>
            {#if vmState === 'stopped' || vmState === 'error'}
                <button class="tb-btn green" onclick={() => node.start()}>▶ Start</button>
            {:else if vmState === 'running'}
                <button class="tb-btn"     onclick={() => node.pause()}>⏸</button>
                <button class="tb-btn red" onclick={() => node.stop()}>⏹ Stop</button>
            {:else if vmState === 'paused'}
                <button class="tb-btn green" onclick={() => node.resume()}>▶ Resume</button>
                <button class="tb-btn red"   onclick={() => node.stop()}>⏹ Stop</button>
            {:else if vmState === 'starting'}
                <span class="starting-label">Starting…</span>
                <button class="tb-btn red" onclick={() => node.stop()}>⏹ Stop</button>
            {/if}
            <button class="tb-btn icon" title="Sync clipboard" onclick={syncClipboard}>📋</button>
            <button class="tb-btn icon" title="Reset size"     onclick={resetSize}>⤡</button>
            <button class="tb-btn icon" title="Fullscreen"     onclick={toggleFullscreen}>⤢</button>
            <button class="tb-btn icon" title="Config"         onclick={() => showConfig = !showConfig}>⚙</button>
        </div>

        <div class="canvas-wrap"
            role="presentation"
            onpointerdown={onPointerDown}
            onpointermove={onPointerMove}
            onpointerup={onPointerUp}
            onpointercancel={onPointerUp}
            onwheel={onWheel}
        >
            <canvas bind:this={canvasEl} class="vm-canvas"></canvas>
            {#if vmState === 'stopped'}
                <div class="overlay-msg">Stopped</div>
            {:else if vmState === 'starting'}
                <div class="overlay-msg"><span class="pulse">Booting…</span></div>
            {:else if vmState === 'error'}
                <div class="overlay-msg error">{vmError || 'Error'}</div>
            {/if}
        </div>

        {#if showConfig}
            <MachineConfig {node} onclose={() => showConfig = false} />
        {/if}
    </div>
    {/if}

    {#if fullscreen}
    <div class="fullscreen-modal" role="application" aria-label="VM fullscreen" onblur={onBlur} tabindex="-1">
        <div class="fs-toolbar">
            <span class="state-dot" style:background={stateColor[vmState] ?? stateColor.stopped}></span>
            <span class="title-label">{node.title ?? 'Machine'}</span>
            <div class="spacer"></div>
            <button class="tb-btn icon" onclick={syncClipboard}>📋 Clipboard</button>
            <button class="tb-btn icon" onclick={toggleFullscreen}>✕ Exit</button>
        </div>
        <div class="fs-canvas-wrap" role="presentation"
            onpointerdown={onPointerDown} onpointerup={onPointerUp}
            onpointermove={onPointerMove} onpointercancel={onPointerUp}
            onwheel={onWheel}>
            <canvas bind:this={fsCanvasEl} class="vm-canvas fs"></canvas>
        </div>
    </div>
    {/if}
</div>


<style>
    .machine-root {
        position: absolute; top: 0; left: 0;
        width: 0; height: 0; pointer-events: none; overflow: visible;
    }
    .machine-root.fs {
        width: 100%; height: 100%; inset: 0;
    }
    .machine-overlay {
        position: absolute; pointer-events: auto;
        display: flex; flex-direction: column;
        background: #0d0d12; border: 1px solid rgba(255,255,255,0.08);
        border-radius: 6px; overflow: visible; z-index: 5;
    }
    .machine-overlay.focused {
        border-color: rgba(110,231,183,0.45);
        box-shadow: 0 0 0 1px rgba(110,231,183,0.15);
    }
    .toolbar {
        display: flex; align-items: center; gap: 5px;
        padding: 4px 8px; height: 32px; flex-shrink: 0;
        background: rgba(255,255,255,0.03);
        border-bottom: 1px solid rgba(255,255,255,0.06);
        box-sizing: border-box;
    }
    .state-dot {
        width: 8px; height: 8px; border-radius: 50%;
        flex-shrink: 0; transition: background 0.3s;
    }
    .title-label { font-size: 11px; color: rgba(226,232,240,0.7); flex-shrink: 0; }
    .spacer { flex: 1; }
    .tb-btn {
        padding: 2px 8px; border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px; background: rgba(255,255,255,0.05);
        color: rgba(226,232,240,0.7); font-size: 10px;
        cursor: pointer; font-family: monospace; white-space: nowrap;
    }
    .tb-btn:hover { background: rgba(255,255,255,0.12); color: #e2e8f0; }
    .tb-btn.green { color: #6ee7b7; border-color: rgba(110,231,183,0.25); }
    .tb-btn.green:hover { background: rgba(110,231,183,0.1); }
    .tb-btn.red   { color: #f472b6; border-color: rgba(244,114,182,0.25); }
    .tb-btn.red:hover { background: rgba(244,114,182,0.08); }
    .tb-btn.icon  { padding: 2px 6px; }
    .starting-label { font-size: 10px; color: #fbbf24; font-family: monospace; }
    .canvas-wrap {
        flex: 1; position: relative; overflow: hidden;
        background: #000; min-height: 0;
        user-select: none; -webkit-user-select: none;
        touch-action: none;
    }
    .vm-canvas {
        width: 100%; height: 100%;
        object-fit: contain; display: block; image-rendering: pixelated;
    }
    .vm-canvas.fs { width: 100%; height: 100%; object-fit: contain; }
    .overlay-msg {
        position: absolute; inset: 0;
        display: flex; align-items: center; justify-content: center;
        background: rgba(0,0,0,0.6); color: rgba(226,232,240,0.35);
        font-size: 13px; font-family: monospace; pointer-events: none;
        padding: 8px; text-align: center;
    }
    .overlay-msg.error { color: #f472b6; font-size: 11px; }
    .pulse { animation: pulse 1.2s ease-in-out infinite; color: #fbbf24; }
    @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
    .fullscreen-modal {
        position: absolute; inset: 0; z-index: 50;
        background: #000; display: flex; flex-direction: column; pointer-events: auto;
    }
    .fs-toolbar {
        display: flex; align-items: center; gap: 8px;
        padding: 6px 12px; height: 38px; flex-shrink: 0;
        background: rgba(13,13,18,0.95);
        border-bottom: 1px solid rgba(255,255,255,0.07);
    }
    .fs-canvas-wrap {
        flex: 1; min-height: 0;
        display: flex; align-items: center; justify-content: center; background: #000;
    }
</style>
