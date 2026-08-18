const ZOOM_KEY = 'knofoo_zoom';
const DEFAULT = 1.0;
const MIN = 0.5;
const MAX = 2.0;
const STEP = 0.1;

class ZoomStore {
    factor = $state(parseFloat(localStorage.getItem(ZOOM_KEY) ?? String(DEFAULT)));

    increase() {
        const next = Math.min(MAX, parseFloat((this.factor + STEP).toFixed(1)));
        this.factor = next;
        localStorage.setItem(ZOOM_KEY, String(next));
    }

    decrease() {
        const next = Math.max(MIN, parseFloat((this.factor - STEP).toFixed(1)));
        this.factor = next;
        localStorage.setItem(ZOOM_KEY, String(next));
    }

    reset() {
        this.factor = DEFAULT;
        localStorage.setItem(ZOOM_KEY, String(DEFAULT));
    }
}

export const zoom = new ZoomStore();
