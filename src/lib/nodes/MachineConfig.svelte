<script lang="ts">
    import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
    import type { MachineNode, MachineConfig } from './MachineNode';
    import Dropdown from '$lib/components/Dropdown.svelte';

    interface Props {
        node: MachineNode;
        onclose: () => void;
    }

    let { node, onclose }: Props = $props();

    // svelte-ignore state_referenced_locally
    const p = node.properties as unknown as MachineConfig;
    // svelte-ignore state_referenced_locally
    let iso_path       = $state(p.iso_path ?? '');
    let ram_mb         = $state(p.ram_mb ?? 1024);
    let cpus           = $state(p.cpus ?? 2);
    let shared_folder  = $state(p.shared_folder ?? '');
    let kb_passthrough = $state(p.input?.keyboard_passthrough ?? true);
    let boot_mode      = $state<'auto' | 'uefi' | 'bios' | 'disk'>(p.boot_mode ?? 'auto');
    let lan            = $state(p.network?.lan ?? true);
    let internet       = $state(p.network?.internet ?? true);
    let port_forwards  = $state([...(p.network?.port_forwards ?? [])]);
    // svelte-ignore state_referenced_locally
    let node_title     = $state(node.title ?? 'Machine');

    async function browseIso() {
        const selected = await dialogOpen({
            title: 'Select ISO or disk image',
            filters: [{ name: 'Disk images', extensions: ['iso', 'img', 'qcow2'] }],
            multiple: false,
        }) as string | null;
        if (selected) iso_path = selected;
    }

    async function browseSharedFolder() {
        const selected = await dialogOpen({
            title: 'Select shared folder',
            directory: true,
            multiple: false,
        }) as string | null;
        if (selected) shared_folder = selected;
    }

    function addPortForward() {
        port_forwards = [...port_forwards, { host: 8080, guest: 80, proto: 'tcp' }];
    }

    function removePortForward(i: number) {
        port_forwards = port_forwards.filter((_, idx) => idx !== i);
    }

    function save() {
        const cfg = node.properties as unknown as MachineConfig;
        cfg.iso_path       = iso_path || null;
        cfg.ram_mb         = ram_mb;
        cfg.cpus           = cpus;
        cfg.shared_folder  = shared_folder || null;
        cfg.input          = { keyboard_passthrough: kb_passthrough };
        cfg.network        = { lan, internet, port_forwards };
        cfg.boot_mode      = boot_mode;
        node.title = node_title;
        (node as any).setDirtyCanvas?.(true, false);
        onclose();
    }
</script>

<div class="config-panel" role="dialog" aria-label="Machine configuration">
    <div class="config-header">
        <span>Machine Config</span>
        <button class="close-btn" onclick={onclose}>×</button>
    </div>

    <div class="config-body">
        <label class="field">
            <span>Name</span>
            <input type="text" bind:value={node_title} placeholder="My Server" />
        </label>

        <label class="field">
            <span>ISO / Image</span>
            <div class="file-row">
                <input type="text" bind:value={iso_path} placeholder="/path/to/image.iso" />
                <button onclick={browseIso}>Browse</button>
            </div>
        </label>

        <div class="field">
            <span>Boot mode</span>
            <Dropdown
                value={boot_mode}
                onchange={(v) => boot_mode = v as typeof boot_mode}
                options={[
                    { value: 'auto', label: 'Auto (detect)' },
                    { value: 'uefi', label: 'UEFI (OVMF, ESP extracted)' },
                    { value: 'bios', label: 'Legacy BIOS (SeaBIOS, El Torito)' },
                    { value: 'disk', label: 'Disk image (boot from .img / .qcow2)' },
                ]}
            />
        </div>

        <div class="row-2">
            <label class="field">
                <span>RAM (MB)</span>
                <input type="number" bind:value={ram_mb} min="128" max="32768" step="256" />
            </label>
            <label class="field">
                <span>CPUs</span>
                <input type="number" bind:value={cpus} min="1" max="16" />
            </label>
        </div>

        <label class="field">
            <span>Shared folder → <code>/workspace</code></span>
            <div class="file-row">
                <input type="text" bind:value={shared_folder} placeholder="(none)" />
                <button onclick={browseSharedFolder}>Browse</button>
            </div>
        </label>

        <fieldset class="section">
            <legend>Network</legend>
            <label class="check">
                <input type="checkbox" bind:checked={lan} />
                <span>LAN</span>
            </label>
            <label class="check">
                <input type="checkbox" bind:checked={internet} />
                <span>Internet (NAT)</span>
            </label>
            {#each port_forwards as pf, i}
                <div class="pf-row">
                    <input type="number" bind:value={pf.host}  min="1" max="65535" />
                    <span>→</span>
                    <input type="number" bind:value={pf.guest} min="1" max="65535" />
                    <Dropdown
                        value={pf.proto}
                        onchange={(v) => pf.proto = v}
                        options={[
                            { value: 'tcp', label: 'TCP' },
                            { value: 'udp', label: 'UDP' },
                        ]}
                    />
                    <button class="rm-btn" onclick={() => removePortForward(i)}>×</button>
                </div>
            {/each}
            <button class="add-pf" onclick={addPortForward}>+ Port forward</button>
        </fieldset>

        <fieldset class="section">
            <legend>Input</legend>
            <label class="check">
                <input type="checkbox" bind:checked={kb_passthrough} />
                <span>Keyboard passthrough</span>
            </label>
        </fieldset>
    </div>

    <div class="config-footer">
        <button class="save-btn" onclick={save}>Save</button>
        <button class="cancel-btn" onclick={onclose}>Cancel</button>
    </div>
</div>

<style>
    .config-panel {
        position: absolute;
        bottom: 0; left: 0; right: 0;
        z-index: 20;
        background: #0f0f1a;
        border-top: 1px solid rgba(255,255,255,0.1);
        display: flex;
        flex-direction: column;
        max-height: 70%;
        overflow: hidden;
    }
    .config-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 7px 12px;
        border-bottom: 1px solid rgba(255,255,255,0.06);
        font-size: 11px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: rgba(226,232,240,0.4);
        flex-shrink: 0;
    }
    .close-btn {
        background: none; border: none;
        color: rgba(226,232,240,0.4); cursor: pointer;
        font-size: 1rem; padding: 0 2px; line-height: 1;
    }
    .close-btn:hover { color: #e2e8f0; }
    .config-body {
        overflow-y: auto; padding: 10px 12px;
        display: flex; flex-direction: column; gap: 8px;
        flex: 1; min-height: 0;
    }
    .field { display: flex; flex-direction: column; gap: 3px; }
    .field span {
        font-size: 10px; color: rgba(226,232,240,0.4);
        text-transform: uppercase; letter-spacing: 0.05em;
    }
    input[type="text"], input[type="number"], select {
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px; color: #e2e8f0;
        font-size: 11px; padding: 4px 7px;
        font-family: monospace; outline: none;
    }
    input:focus, select:focus { border-color: rgba(110,231,183,0.4); }
    .file-row { display: flex; gap: 5px; }
    .file-row input { flex: 1; }
    .file-row button {
        padding: 4px 8px; border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px; background: rgba(255,255,255,0.05);
        color: rgba(226,232,240,0.6); font-size: 10px; cursor: pointer;
    }
    .row-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
    .section {
        border: 1px solid rgba(255,255,255,0.07);
        border-radius: 5px; padding: 8px 10px; margin: 0;
    }
    .section legend {
        font-size: 10px; color: rgba(226,232,240,0.35);
        text-transform: uppercase; letter-spacing: 0.06em; padding: 0 4px;
    }
    .check {
        display: flex; align-items: center; gap: 7px;
        font-size: 11px; color: rgba(226,232,240,0.65); cursor: pointer; padding: 2px 0;
    }
    .check input { accent-color: #6ee7b7; }
    .pf-row { display: flex; align-items: center; gap: 5px; margin-bottom: 4px; }
    .pf-row input { width: 60px; }
    .pf-row span { color: rgba(226,232,240,0.4); font-size: 11px; }
    .pf-row select { width: 55px; }
    .rm-btn {
        background: none; border: none;
        color: rgba(244,114,182,0.5); cursor: pointer; font-size: 14px;
    }
    .add-pf {
        background: none; border: 1px dashed rgba(255,255,255,0.1);
        border-radius: 4px; color: rgba(226,232,240,0.35);
        font-size: 10px; padding: 3px 8px; cursor: pointer; width: 100%;
    }
    .config-footer {
        display: flex; gap: 8px; padding: 8px 12px;
        border-top: 1px solid rgba(255,255,255,0.06);
        justify-content: flex-end; flex-shrink: 0;
    }
    .save-btn {
        padding: 4px 14px; border: 1px solid rgba(110,231,183,0.3);
        border-radius: 4px; background: rgba(110,231,183,0.1);
        color: #6ee7b7; font-size: 11px; cursor: pointer;
    }
    .cancel-btn {
        padding: 4px 10px; border: 1px solid rgba(255,255,255,0.1);
        border-radius: 4px; background: none;
        color: rgba(226,232,240,0.45); font-size: 11px; cursor: pointer;
    }
</style>
