import { readDir } from '@tauri-apps/plugin-fs';

export interface Entry {
    name: string;
    isDirectory: boolean;
}

export class VaultDirectory {
    entries = $state<Entry[]>([]);
    error = $state('');
    loading = $state(false);

    async load(vaultPath: string) {
        this.error = '';
        this.entries = [];
        if (!vaultPath) return;

        this.loading = true;
        try {
            const result = await readDir(vaultPath);
            this.entries = result
                .map((e) => ({ name: e.name ?? '', isDirectory: e.isDirectory }))
                .sort(compareEntries);
        } catch (e) {
            this.error = String(e);
        } finally {
            this.loading = false;
        }
    }
}

function compareEntries(a: Entry, b: Entry): number {
    if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
    return a.name.localeCompare(b.name);
}