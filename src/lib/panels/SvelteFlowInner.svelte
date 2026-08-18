<script lang="ts">
    import { onMount } from 'svelte';
    import {
        SvelteFlow, Controls, Background, BackgroundVariant,
        useSvelteFlow, type Node, type Edge,
    } from '@xyflow/svelte';

    let {
        nodes = $bindable(),
        edges = $bindable(),
        nodeTypes,
        onready,
    }: {
        nodes: Node[];
        edges: Edge[];
        nodeTypes: Record<string, any>;
        onready: (getViewport: () => { x: number; y: number; zoom: number }) => void;
    } = $props();

    const { getViewport } = useSvelteFlow();

    onMount(() => {
        onready(getViewport);
    });
</script>

<SvelteFlow bind:nodes bind:edges {nodeTypes} fitView colorMode="dark">
    <Background variant={BackgroundVariant.Dots} />
    <Controls position="bottom-right" showLock={false} />
</SvelteFlow>
