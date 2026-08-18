import * as THREE from 'three';

const COLORS = [0x6ee7b7, 0x818cf8, 0xf472b6, 0xfbbf24, 0x38bdf8];

export interface NodeScene {
    id: string;
    scene: THREE.Scene;
    camera: THREE.PerspectiveCamera;
    mesh: THREE.Mesh;
    canvas2d: HTMLCanvasElement;
    ctx2d: CanvasRenderingContext2D;
    w: number;
    h: number;
    visible: boolean; // set by caller — false = skip render this frame
}

const OFFSCREEN_SIZE = 256;

export class SharedWebGLRenderer {
    nodes = new Map<string, NodeScene>();
    private renderer: THREE.WebGLRenderer;
    private offscreen: HTMLCanvasElement;
    private geo: THREE.TorusKnotGeometry;
    private animId = 0;

    constructor() {
        this.offscreen = document.createElement('canvas');
        this.offscreen.width  = OFFSCREEN_SIZE;
        this.offscreen.height = OFFSCREEN_SIZE;

        this.renderer = new THREE.WebGLRenderer({
            canvas: this.offscreen,
            antialias: true,
            alpha: true,
        });
        this.renderer.setPixelRatio(1);
        this.renderer.setSize(OFFSCREEN_SIZE, OFFSCREEN_SIZE, false);
        this.renderer.setClearColor(0x000000, 0);
        this.renderer.autoClear = true;

        this.geo = new THREE.TorusKnotGeometry(0.45, 0.15, 80, 20);
    }

    register(id: string, colorIndex: number): NodeScene {
        const scene  = new THREE.Scene();
        const camera = new THREE.PerspectiveCamera(60, 1, 0.1, 100);
        camera.position.set(0, 0, 2.2);

        const mat = new THREE.MeshStandardMaterial({
            color: COLORS[colorIndex % COLORS.length],
            metalness: 0.4,
            roughness: 0.3,
        });
        const mesh = new THREE.Mesh(this.geo, mat);
        scene.add(mesh);

        const light = new THREE.DirectionalLight(0xffffff, 2);
        light.position.set(2, 3, 4);
        scene.add(light);
        scene.add(new THREE.AmbientLight(0x334455, 3));

        const canvas2d = document.createElement('canvas');
        canvas2d.style.cssText = 'width:100%;height:100%;display:block;';
        const ctx2d = canvas2d.getContext('2d')!;

        const entry: NodeScene = { id, scene, camera, mesh, canvas2d, ctx2d, w: 0, h: 0, visible: false };
        this.nodes.set(id, entry);
        return entry;
    }

    unregister(id: string) {
        const entry = this.nodes.get(id);
        if (entry) {
            (entry.mesh.material as THREE.Material).dispose();
            this.nodes.delete(id);
        }
    }

    start() {
        const tick = () => {
            this.animId = requestAnimationFrame(tick);
            this.renderAll();
        };
        tick();
    }

    private renderAll() {
        const renderer = this.renderer;

        for (const node of this.nodes.values()) {
            // Skip if caller marked invisible or too small
            if (!node.visible || node.w < 4 || node.h < 4) continue;

            const aspect = node.w / node.h;
            if (node.camera.aspect !== aspect) {
                node.camera.aspect = aspect;
                node.camera.updateProjectionMatrix();
            }

            node.mesh.rotation.x += 0.008;
            node.mesh.rotation.y += 0.012;

            renderer.render(node.scene, node.camera);

            if (node.canvas2d.width !== node.w || node.canvas2d.height !== node.h) {
                node.canvas2d.width  = node.w;
                node.canvas2d.height = node.h;
            }
            node.ctx2d.clearRect(0, 0, node.w, node.h);
            node.ctx2d.drawImage(this.offscreen, 0, 0, node.w, node.h);
        }
    }

    stop() {
        cancelAnimationFrame(this.animId);
        this.renderer.dispose();
        this.geo.dispose();
    }
}
