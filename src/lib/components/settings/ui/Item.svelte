<script lang="ts">
    let { label, children } = $props<{ label: string; children?: any }>();
</script>

<div class="item">
    <span class="item-label">{label}</span>
    <div class="item-control">
        {@render children?.()}
    </div>
</div>

<style>
    .item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.5rem 0.75rem;
        border-radius: 6px;
        background: rgba(255, 255, 255, 0.03);
    }

    .item-label {
        font-size: 1rem;
        color: rgba(226, 232, 240, 0.85);
    }

    .item-control {
        display: flex;
        align-items: center;
    }
</style>
