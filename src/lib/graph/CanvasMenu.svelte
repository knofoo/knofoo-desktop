<script lang="ts">
    interface Props {
        x: number;
        y: number;
        onaddcontent: () => void;
        onaddvalidator: () => void;
        onaddmodule: () => void;
        onaddmachine: () => void;
        onclose: () => void;
    }

    let { x, y, onaddcontent, onaddvalidator, onaddmodule, onaddmachine, onclose }: Props = $props();

    $effect(() => {
        function onKd(e: KeyboardEvent) { if (e.key === 'Escape') onclose(); }
        window.addEventListener('keydown', onKd);
        return () => window.removeEventListener('keydown', onKd);
    });
</script>

<div class="canvas-menu" style="left:{x}px;top:{y}px" role="menu">
    <div class="menu-label">Add node</div>
    <button role="menuitem" onclick={() => { onaddcontent(); onclose(); }}>
        <span class="item-icon" style="color:rgba(59,130,246,0.8)">◇</span>
        Content
        <span class="item-hint">markdown · image · video</span>
    </button>
    <button role="menuitem" onclick={() => { onaddvalidator(); onclose(); }}>
        <span class="item-icon" style="color:rgba(129,140,248,0.8)">⬡</span>
        Validator
        <span class="item-hint">question · hint · answer</span>
    </button>
    <button role="menuitem" onclick={() => { onaddmodule(); onclose(); }}>
        <span class="item-icon" style="color:rgba(14,165,233,0.8)">⊞</span>
        Module
        <span class="item-hint">sub-graph · reusable unit</span>
    </button>
    <button role="menuitem" onclick={() => { onaddmachine(); onclose(); }}>
        <span class="item-icon" style="color:rgba(110,231,183,0.8)">⬡</span>
        Machine
        <span class="item-hint">VM · boots any ISO</span>
    </button>
</div>

<style>
    .canvas-menu {
        position: absolute;
        z-index: 200;
        background: #16162a;
        border: 1px solid rgba(255,255,255,0.12);
        border-radius: 8px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.6);
        padding: 4px;
        min-width: 200px;
        pointer-events: auto;
    }

    .menu-label {
        font-size: 0.7rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(226,232,240,0.25);
        padding: 4px 10px 2px;
    }

    button[role="menuitem"] {
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        padding: 7px 10px;
        background: none;
        border: none;
        border-radius: 5px;
        color: rgba(226,232,240,0.8);
        font-size: 0.9rem;
        text-align: left;
        cursor: pointer;
        transition: background 0.1s;
    }

    button[role="menuitem"]:hover {
        background: rgba(14,165,233,0.12);
        color: #fff;
    }

    .item-icon {
        font-size: 0.8rem;
        flex-shrink: 0;
    }

    .item-hint {
        margin-left: auto;
        font-size: 0.72rem;
        color: rgba(226,232,240,0.25);
        font-family: monospace;
    }
</style>
