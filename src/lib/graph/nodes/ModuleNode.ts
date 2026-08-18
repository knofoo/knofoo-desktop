import { LGraphNode, LiteGraph } from '@comfyorg/litegraph';

export class ModuleNode extends LGraphNode {
    static override title = 'Module';
    static override category = 'knofoo';

    constructor() {
        super('Module');
        this.size = [280, 90];
        this.addInput('in', '*');
        this.addOutput('out', '*');
        this.properties = {
            path:        '',
            title:       'Untitled Module',
            description: '',
            color:       '#0ea5e9',
            bgcolor:     '#0c1a2e',
            borderRadius: 8,
        };
        this.color   = '#0ea5e9';
        this.bgcolor = '#0c1a2e';
    }

    override onDrawBackground(ctx: CanvasRenderingContext2D) {
        const r = Number(this.properties.borderRadius ?? 8);
        const w = this.size[0];
        const h = this.size[1];

        ctx.fillStyle = String(this.properties.bgcolor ?? '#0c1a2e');
        ctx.beginPath();
        ctx.roundRect(0, 0, w, h, [0, 0, r, r]);
        ctx.fill();

        // description text
        const desc = String(this.properties.description ?? '');
        if (desc) {
            ctx.fillStyle = 'rgba(186,230,253,0.55)';
            ctx.font = '11px sans-serif';
            const maxW = w - 16;
            // simple word-wrap one line
            let line = '';
            for (const word of desc.split(' ')) {
                const test = line ? `${line} ${word}` : word;
                if (ctx.measureText(test).width > maxW && line) break;
                line = test;
            }
            ctx.fillText(line + (line !== desc ? '…' : ''), 8, 22);
        }

        // path badge bottom-right
        if (this.properties.path) {
            const name = String(this.properties.path).split(/[\\/]/).at(-1) ?? '';
            ctx.fillStyle = 'rgba(14,165,233,0.18)';
            const bw = Math.min(ctx.measureText(name).width + 12, w - 16);
            ctx.beginPath();
            ctx.roundRect(w - bw - 6, h - 18, bw, 14, 3);
            ctx.fill();
            ctx.fillStyle = 'rgba(186,230,253,0.45)';
            ctx.font = '9px monospace';
            ctx.fillText(name, w - bw - 2, h - 7);
        }
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
        ctx.fillStyle = String(this.properties.color ?? '#0ea5e9');
        ctx.beginPath();
        ctx.roundRect(-1, -1, w + 2, title_height + 1, [r, r, 0, 0]);
        ctx.fill();

        // module icon in title
        ctx.fillStyle = 'rgba(255,255,255,0.7)';
        ctx.font = `${title_height * 0.55}px sans-serif`;
        ctx.fillText('⊞', 6, title_height * 0.72);
    }
}

export function registerModuleNode() {
    if (!(LiteGraph as any).registered_node_types?.['knofoo/module']) {
        LiteGraph.registerNodeType('knofoo/module', ModuleNode);
    }
}
