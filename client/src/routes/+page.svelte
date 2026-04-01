<script>
    import Upload from '$lib/Upload.svelte';
</script>

<svelte:head>
    <title>Upload Video</title>
</svelte:head>

<main>
    <h1>Video Streaming</h1>
    <p>Upload a video to get a shareable link instantly.</p>
    <Upload />
</main>

<style>
    main {
        max-width: 640px;
        margin: 0 auto;
        padding: 48px 24px;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
        color: #eee;
    }

    h1 {
        font-size: 28px;
        margin-bottom: 8px;
    }

    p {
        color: #aaa;
        margin-bottom: 32px;
    }
</style>
