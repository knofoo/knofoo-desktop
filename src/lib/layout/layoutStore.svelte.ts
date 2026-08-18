export type PanelMode = 'fixed' | 'dynamic';

export interface PanelState {
    visible: boolean;
    width: number;
    minWidth: number;
    minHeight: number;
    mode: PanelMode;
}

const DIVIDER_PX = 4;

export class LayoutStore {
    explorer = $state<PanelState>({ visible: true, width: 220, minWidth: 120, minHeight: 0, mode: 'dynamic' });
    graph    = $state<PanelState>({ visible: true, width: 0,   minWidth: 200, minHeight: 0, mode: 'dynamic' });
    settings = $state<PanelState>({ visible: true, width: 260, minWidth: 120, minHeight: 0, mode: 'dynamic' });

    containerWidth = $state(0);

    private visibleDividers(): number {
        return Math.max(0, [this.explorer, this.graph, this.settings].filter(p => p.visible).length - 1);
    }

    private graphWidth(): number {
        return Math.max(0,
            this.containerWidth
            - (this.explorer.visible ? this.explorer.width : 0)
            - (this.settings.visible ? this.settings.width : 0)
            - this.visibleDividers() * DIVIDER_PX
        );
    }

    toggleExplorer() {
        if (this.explorer.visible) {
            const freed = this.explorer.width + DIVIDER_PX;
            if (!this.graph.visible && this.settings.visible && this.settings.mode === 'dynamic') {
                this.settings.width += freed;
            }
            this.explorer.visible = false;
        } else {
            this.explorer.visible = true;
            const freed = this.explorer.width + DIVIDER_PX;
            if (!this.graph.visible && this.settings.visible && this.settings.mode === 'dynamic') {
                this.settings.width = Math.max(this.settings.minWidth, this.settings.width - freed);
            }
        }
    }

    toggleGraph() {
        if (this.graph.visible) {
            const gw = this.graphWidth();
            const leftDyn  = this.explorer.visible && this.explorer.mode === 'dynamic';
            const rightDyn = this.settings.visible && this.settings.mode === 'dynamic';
            if (leftDyn && rightDyn) {
                this.explorer.width += Math.floor(gw / 2);
                this.settings.width += Math.ceil(gw / 2);
            } else if (leftDyn)  { this.explorer.width += gw; }
            else if (rightDyn)   { this.settings.width += gw; }
            this.graph.visible = false;
        } else {
            this.graph.visible = true;
            const reclaim = this.containerWidth
                - (this.explorer.visible ? this.explorer.width : 0)
                - (this.settings.visible ? this.settings.width : 0)
                - this.visibleDividers() * DIVIDER_PX;
            const needed = this.graph.minWidth - reclaim;
            if (needed > 0) {
                const leftDyn  = this.explorer.visible && this.explorer.mode === 'dynamic';
                const rightDyn = this.settings.visible && this.settings.mode === 'dynamic';
                if (leftDyn && rightDyn) {
                    this.explorer.width = Math.max(this.explorer.minWidth, this.explorer.width - Math.floor(needed / 2));
                    this.settings.width = Math.max(this.settings.minWidth, this.settings.width - Math.ceil(needed / 2));
                } else if (leftDyn)  { this.explorer.width = Math.max(this.explorer.minWidth, this.explorer.width - needed); }
                else if (rightDyn)   { this.settings.width = Math.max(this.settings.minWidth, this.settings.width - needed); }
            }
        }
    }

    toggleSettings() {
        if (this.settings.visible) {
            const freed = this.settings.width + DIVIDER_PX;
            if (!this.graph.visible && this.explorer.visible && this.explorer.mode === 'dynamic') {
                this.explorer.width += freed;
            }
            this.settings.visible = false;
        } else {
            this.settings.visible = true;
            const freed = this.settings.width + DIVIDER_PX;
            if (!this.graph.visible && this.explorer.visible && this.explorer.mode === 'dynamic') {
                this.explorer.width = Math.max(this.explorer.minWidth, this.explorer.width - freed);
            }
        }
    }

    setExplorerWidth(w: number) { this.explorer.width = Math.max(this.explorer.minWidth, w); }
    setSettingsWidth(w: number) { this.settings.width = Math.max(this.settings.minWidth, w); }
}

export const layout = new LayoutStore();
