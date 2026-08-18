import { LGraphNode, LiteGraph } from '@comfyorg/litegraph';

export type ValidatorStrategy = 'regex' | 'nlp' | 'choice' | 'hash';

export class ValidatorNode extends LGraphNode {
    static override title = 'Validator';
    static override category = 'knofoo';

    constructor() {
        super('Validator');
        this.size = [260, 120];
        this.addInput('in', 'data');
        this.addOutput('result', 'boolean');
        this.properties = {
            strategy:     'regex' as ValidatorStrategy,
            question:     '',
            hint:         '',
            color:        '#818cf8',
            bgcolor:      '#1a1a2e',
            borderRadius: 8,
        };
        this.color   = '#818cf8';
        this.bgcolor = '#1a1a2e';
    }

    override onDrawBackground(ctx: CanvasRenderingContext2D) {
        const r = Number(this.properties.borderRadius ?? 8);
        const w = this.size[0];
        const h = this.size[1];
        ctx.fillStyle = String(this.properties.bgcolor ?? '#1a1a2e');
        ctx.beginPath();
        ctx.roundRect(0, 0, w, h, [0, 0, r, r]);
        ctx.fill();

        // strategy badge
        const strategy = String(this.properties.strategy ?? 'regex');
        ctx.fillStyle = 'rgba(129,140,248,0.25)';
        ctx.beginPath();
        ctx.roundRect(8, 8, 52, 16, 4);
        ctx.fill();
        ctx.fillStyle = '#818cf8';
        ctx.font = '9px monospace';
        ctx.fillText(strategy, 12, 20);
    }

    override onDrawTitleBar(
        ctx: CanvasRenderingContext2D,
        title_height: number,
        size: [number, number],
        _scale: number,
        _fgcolor: string,
    ) {
        const r = Number(this.properties.borderRadius ?? 8);
        const w = size[0];
        ctx.fillStyle = String(this.properties.color ?? '#818cf8');
        ctx.beginPath();
        ctx.roundRect(-1, -1, w + 2, title_height + 1, [r, r, 0, 0]);
        ctx.fill();
    }
}

export function registerValidatorNode() {
    if (!(LiteGraph as any).registered_node_types?.['knofoo/validator']) {
        LiteGraph.registerNodeType('knofoo/validator', ValidatorNode);
    }
}
