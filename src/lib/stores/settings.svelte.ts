export interface SettingsPaths {
    vault: string;
}

export interface Settings {
    paths: SettingsPaths;
}

export const settings = $state<Settings>({
    paths: {
        vault: ''
    }
})

export function setVault(path: string) {
    settings.paths.vault = path;
    persist();
}

export function clearVault() {
    settings.paths.vault = '';
    persist();
}

function persist() {
    localStorage.setItem('knofoo-settings', JSON.stringify(settings));
}

export function hydrateSettings() {
    const raw = localStorage.getItem('knofoo-settings');
    if (raw) {
        const parsed = JSON.parse(raw) as Settings;
        settings.paths.vault = parsed.paths.vault;
    }
}