<script>
    import { page } from '$app/stores';
    import VideoPlayer from '$lib/VideoPlayer.svelte';

    const token = $derived($page.params.token);
</script>

<svelte:head>
    <title>Watch Video</title>
</svelte:head>

<main>
    <VideoPlayer {token} />
    <div class="share-section">
        <p>Share this video:</p>
        <input type="text" readonly value={globalThis.location?.href ?? ''} onclick={(e) => e.target.select()} />
    </div>
</main>

<style>
    main {
        max-width: 960px;
        margin: 0 auto;
        padding: 32px 24px;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
        color: #eee;
    }

    .share-section {
        margin-top: 24px;
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .share-section p {
        color: #aaa;
        font-size: 14px;
        margin: 0;
        white-space: nowrap;
    }

    .share-section input {
        flex: 1;
        padding: 8px 12px;
        background: #222;
        border: 1px solid #444;
        border-radius: 6px;
        color: #eee;
        font-size: 14px;
    }
</style>
