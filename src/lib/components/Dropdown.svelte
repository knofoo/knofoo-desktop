<script lang="ts">
    interface Option {
        value: string;
        label: string;
    }

    interface Props {
        value: string;
        options: Option[];
        onchange: (value: string) => void;
        placeholder?: string;
        disabled?: boolean;
    }

    let { value, options, onchange, placeholder = 'Select…', disabled = false }: Props = $props();

    let open = $state(false);
    let rootEl: HTMLDivElement | undefined = $state();

    const current = $derived(options.find(o => o.value === value));
    const label   = $derived(current?.label ?? placeholder);

    function toggle() {
        if (disabled) return;
        open = !open;
    }

    function pick(v: string) {
        onchange(v);
        open = false;
    }

    $effect(() => {
        if (!open) return;
        function onDocClick(e: MouseEvent) {
            if (rootEl && !rootEl.contains(e.target as Node)) open = false;
        }
        function onKey(e: KeyboardEvent) {
            if (e.key === 'Escape') open = false;
        }
        // defer to next tick so the click that opened doesn't immediately close
        const t = setTimeout(() => {
            window.addEventListener('mousedown', onDocClick);
            window.addEventListener('keydown', onKey);
        }, 0);
        return () => {
            clearTimeout(t);
            window.removeEventListener('mousedown', onDocClick);
            window.removeEventListener('keydown', onKey);
        };
    });
</script>

<div class="dd" class:disabled bind:this={rootEl}>
    <button class="dd-trigger" type="button" onclick={toggle} disabled={disabled} aria-haspopup="listbox" aria-expanded={open}>
        <span class="dd-label">{label}</span>
        <span class="dd-arrow" class:open>▾</span>
    </button>
    {#if open}
        <ul class="dd-menu" role="listbox">
            {#each options as opt (opt.value)}
                <li>
                    <button
                        type="button"
                        class="dd-option"
                        class:selected={opt.value === value}
                        role="option"
                        aria-selected={opt.value === value}
                        onclick={() => pick(opt.value)}
                    >
                        {opt.label}
                    </button>
                </li>
            {/each}
        </ul>
    {/if}
</div>

<style>
    .dd {
        position: relative;
        display: inline-block;
        font-family: inherit;
    }

    .dd-trigger {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        width: 100%;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 4px;
        color: #e2e8f0;
        font-family: inherit;
        font-size: 0.85rem;
        padding: 4px 10px;
        cursor: pointer;
        text-align: left;
    }
    .dd-trigger:hover:not(:disabled) {
        border-color: rgba(255, 255, 255, 0.2);
    }
    .dd-trigger:focus {
        outline: none;
        border-color: rgba(110, 231, 183, 0.4);
    }
    .dd.disabled .dd-trigger {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .dd-label {
        flex: 1;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .dd-arrow {
        color: rgba(226, 232, 240, 0.5);
        font-size: 0.7rem;
        transition: transform 0.15s;
        flex-shrink: 0;
    }
    .dd-arrow.open {
        transform: rotate(180deg);
    }

    .dd-menu {
        position: absolute;
        top: calc(100% + 2px);
        left: 0;
        right: 0;
        z-index: 1000;
        list-style: none;
        margin: 0;
        padding: 4px;
        background: #14141c;
        border: 1px solid rgba(255, 255, 255, 0.12);
        border-radius: 5px;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
        max-height: 240px;
        overflow-y: auto;
    }

    .dd-option {
        display: block;
        width: 100%;
        background: none;
        border: none;
        color: rgba(226, 232, 240, 0.85);
        font-family: inherit;
        font-size: 0.85rem;
        padding: 5px 10px;
        text-align: left;
        cursor: pointer;
        border-radius: 3px;
    }
    .dd-option:hover {
        background: rgba(255, 255, 255, 0.08);
        color: #e2e8f0;
    }
    .dd-option.selected {
        background: rgba(225, 80, 35, 0.18);
        color: #e2e8f0;
    }
</style>
