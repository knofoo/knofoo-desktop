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