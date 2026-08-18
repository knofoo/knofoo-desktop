import { LGraphNode, LiteGraph } from '@comfyorg/litegraph';

export class ContentNode extends LGraphNode {
    static override title = 'Content';
    static override category = 'knofoo';

    constructor() {
        super('Content');
        this.size = [260, 160];
        this.addOutput('out', 'data');
        this.properties = {
            content:      '',
            color:        '#3b82f6',   // title bar color (accent)
            bgcolor:      '#1e1e2e',   // body background
            borderRadius: 8,
        };
        // Apply LiteGraph native color props
        this.color   = '#3b82f6';
        this.bgcolor = '#1e1e2e';
    }

    override onDrawBackground(ctx: CanvasRenderingContext2D) {
        const r   = Number(this.properties.borderRadius ?? 8);
        const w   = this.size[0];
        const h   = this.size[1];
        ctx.fillStyle = String(this.properties.bgcolor ?? '#1e1e2e');
        ctx.beginPath();
        ctx.roundRect(0, 0, w, h, [0, 0, r, r]);
        ctx.fill();
    }

    override onDrawTitleBar(
        ctx: CanvasRenderingContext2D,
        title_height: number,
        size: [number, number],
        scale: number,
        _fgcolor: string,
    ) {
        const r = Number(this.properties.borderRadius ?? 8);
        const w = size[0];
        ctx.fillStyle = String(this.properties.color ?? '#3b82f6');
        ctx.beginPath();
        ctx.roundRect(-1, -1, w + 2, title_height + 1, [r, r, 0, 0]);
        ctx.fill();
    }
}

export function registerContentNode() {
    // Avoid duplicate registration on HMR
    if (!(LiteGraph as any).registered_node_types?.['knofoo/content']) {
        LiteGraph.registerNodeType('knofoo/content', ContentNode);
    }
}
