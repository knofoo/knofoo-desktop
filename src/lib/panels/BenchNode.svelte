<script lang="ts">
    import { Handle, Position } from '@xyflow/svelte';
    import type { SharedWebGLRenderer } from './SharedWebGL';

    let {
        id,
        data,
    }: {
        id: string;
        data: { label: string; color: string; progress: number; renderer: SharedWebGLRenderer; index: number };
    } = $props();

    let webglSlot: HTMLDivElement | undefined = $state();

    $effect(() => {
        if (!webglSlot || !data.renderer) return;
        const ns = data.renderer.nodes.get(id);
        if (!ns) return;
        webglSlot.appendChild(ns.canvas2d);
        return () => {
            ns.canvas2d.remove();
        };
    });
</script>

<div class="bench-node">
    <Handle type="target" position={Position.Left} />

    <div class="card-header">
        <span class="badge" style="background:{data.color}">{data.label[0]}</span>
        <span class="title">{data.label}</span>
        <span class="idx">#{data.index}</span>
    </div>

    <div class="webgl-slot" bind:this={webglSlot}></div>

    <div class="card-footer">
        <div class="progress-track">
            <div class="progress-fill" style="width:{data.progress}%"></div>
        </div>
        <span class="progress-label">{data.progress}%</span>
    </div>

    <Handle type="source" position={Position.Right} />
</div>

<style>
    .bench-node {
        width: 200px;
        background: #1e1e2e;
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 8px;
        overflow: hidden;
        font-family: monospace;
        font-size: 11px;
        color: #e2e8f0;
    }

    .card-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 5px 8px;
        height: 28px;
        box-sizing: border-box;
        background: rgba(255,255,255,0.04);
        border-bottom: 1px solid rgba(255,255,255,0.07);
    }

    .badge {
        width: 18px;
        height: 18px;
        border-radius: 4px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        font-size: 10px;
        color: #000;
        flex-shrink: 0;
    }

    .title {
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .idx {
        color: rgba(255,255,255,0.25);
        font-size: 9px;
    }

    .webgl-slot {
        width: 100%;
        height: 80px;
        background: #0a0a14;
        overflow: hidden;
    }

    .card-footer {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 5px 8px;
        height: 22px;
        box-sizing: border-box;
    }

    .progress-track {
        flex: 1;
        height: 4px;
        background: rgba(255,255,255,0.1);
        border-radius: 2px;
        overflow: hidden;
    }

    .progress-fill {
        height: 100%;
        background: linear-gradient(90deg, #6ee7b7, #818cf8);
        border-radius: 2px;
    }

    .progress-label {
        color: rgba(255,255,255,0.35);
        font-size: 9px;
        width: 24px;
        text-align: right;
    }
</style>
