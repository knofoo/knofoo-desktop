type LogEntry = { t: string; msg: string; kind: 'info' | 'error' };

class LogStore {
    entries = $state<LogEntry[]>([]);

    log(msg: string) {
        const t = new Date().toTimeString().slice(0, 8);
        this.entries = [...this.entries.slice(-99), { t, msg, kind: 'info' }];
    }
    error(msg: string) {
        const t = new Date().toTimeString().slice(0, 8);
        this.entries = [...this.entries.slice(-99), { t, msg, kind: 'error' }];
    }
    clear() { this.entries = []; }
}

export const logStore = new LogStore();
