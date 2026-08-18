import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';

const VAULT_KEY = 'knofoo_vault_path';

export interface KnofooConfig {
    knofoo: string;
    paths: {
        graphs: string;
        modules: string;
        notes: string;
        assets: string;
    };
    editor: {
        snapToGrid: boolean;
        gridSize: number;
    };
    assetResolution?: string;
}

const DEFAULT_CONFIG: KnofooConfig = {
    knofoo: '0.1.0',
    paths: {
        graphs:  '.knofoo/graphs',
        modules: '.knofoo/modules',
        notes:   '.knofoo/notes',
        assets:  '.knofoo/assets',
    },
    editor: {
        snapToGrid: false,
        gridSize: 10,
    },
};

// Reactive state held inside a $state object — accessing s.path / s.config
// from anywhere produces dependency tracking so $effect / $derived re-run.
const s = $state<{ path: string | null; config: KnofooConfig }>({
    path: localStorage.getItem(VAULT_KEY),
    config: DEFAULT_CONFIG,
});

async function loadConfig() {
    const p = s.path;
    if (!p) return;
    try {
        const raw = await invoke<string>('read_config', { vaultPath: p });
        s.config = JSON.parse(raw);
    } catch {
        s.config = DEFAULT_CONFIG;
    }
}

async function saveConfig(updated: KnofooConfig) {
    const p = s.path;
    if (!p) return;
    await invoke('write_config', { vaultPath: p, config: JSON.stringify(updated, null, 2) });
    s.config = updated;
}

function resolvePath(relative: string): string {
    return s.path ? `${s.path}/${relative}` : relative;
}

async function set(newPath: string) {
    localStorage.setItem(VAULT_KEY, newPath);
    s.path = newPath;
    // Make sure .knofoo and config.json exist before reading config.
    await invoke('init_vault', { path: newPath }).catch(() => {});
    await loadConfig();
}

async function pick(): Promise<string | null> {
    const selected = await dialogOpen({
        title: 'Select vault folder',
        directory: true,
        multiple: false,
    }) as string | null;
    if (!selected) return null;
    await set(selected);
    return selected;
}

function clear() {
    localStorage.removeItem(VAULT_KEY);
    s.path = null;
    s.config = DEFAULT_CONFIG;
}

// load config on init if vault already set
if (s.path) loadConfig();

export const vault = {
    get path() { return s.path; },
    get config() { return s.config; },
    get isConfigured() { return s.path !== null; },
    resolvePath,
    set,
    pick,
    clear,
    saveConfig,
};
