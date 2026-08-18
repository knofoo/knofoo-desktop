<script lang="ts">
    let { title, children } = $props<{ title: string; children?: any }>();
</script>

<section class="section">
    <h2 class="section-title">{title}</h2>
    <div class="section-body">
        {@render children?.()}
    </div>
</section>

<style>
    .section {
        margin-bottom: 2rem;
    }

    .section-title {
        font-size: 0.85rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(226, 232, 240, 0.4);
        margin-bottom: 0.75rem;
    }

    .section-body {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
</style>
