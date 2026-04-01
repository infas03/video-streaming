<script>
    import Hls from 'hls.js';
    import { onMount, onDestroy } from 'svelte';

    let { token } = $props();

    let videoElement = $state(null);
    let status = $state('loading');
    let eventSource = null;
    let hlsInstance = null;

    onMount(() => {
        loadVideoMetadata();
    });

    onDestroy(() => {
        if (eventSource) eventSource.close();
        if (hlsInstance) hlsInstance.destroy();
    });

    async function loadVideoMetadata() {
        try {
            const response = await fetch(`/api/videos/${token}`);
            if (!response.ok) {
                status = 'error';
                return;
            }

            const data = await response.json();
            status = data.status;

            startRawPlayback();

            if (!data.hls_ready) {
                listenForHlsReady();
            } else {
                switchToHls();
            }
        } catch {
            status = 'error';
        }
    }

    function startRawPlayback() {
        if (!videoElement) return;
        videoElement.src = `/api/videos/${token}/raw`;
        videoElement.load();
    }

    function listenForHlsReady() {
        eventSource = new EventSource(`/api/videos/${token}/status`);

        eventSource.onmessage = (event) => {
            const data = JSON.parse(event.data);
            status = data.status;

            if (data.hls_ready) {
                eventSource.close();
                eventSource = null;
                switchToHls();
            }
        };

        eventSource.onerror = () => {
            eventSource.close();
            eventSource = null;
        };
    }

    function switchToHls() {
        if (!videoElement) return;

        const manifestUrl = `/api/videos/${token}/manifest.m3u8`;
        const currentTime = videoElement.currentTime;
        const wasPlaying = !videoElement.paused;

        if (Hls.isSupported()) {
            hlsInstance = new Hls();
            hlsInstance.loadSource(manifestUrl);
            hlsInstance.attachMedia(videoElement);
            hlsInstance.on(Hls.Events.MANIFEST_PARSED, () => {
                videoElement.currentTime = currentTime;
                if (wasPlaying) videoElement.play();
            });
        } else if (videoElement.canPlayType('application/vnd.apple.mpegurl')) {
            videoElement.src = manifestUrl;
            videoElement.addEventListener('loadedmetadata', () => {
                videoElement.currentTime = currentTime;
                if (wasPlaying) videoElement.play();
            }, { once: true });
        }

        status = 'done';
    }
</script>

<div class="player-container">
    {#if status === 'error'}
        <div class="player-message">
            <p>Video not found or failed to load.</p>
        </div>
    {:else}
        <video
            bind:this={videoElement}
            controls
            playsinline
            autoplay
        >
            Your browser does not support video playback.
        </video>

        {#if status === 'transcoding'}
            <div class="status-badge">Optimizing video...</div>
        {/if}
    {/if}
</div>

<style>
    .player-container {
        position: relative;
        max-width: 960px;
        margin: 0 auto;
    }

    video {
        width: 100%;
        border-radius: 8px;
        background: #000;
    }

    .status-badge {
        position: absolute;
        top: 12px;
        right: 12px;
        padding: 6px 12px;
        background: rgba(0, 0, 0, 0.7);
        color: #4f8eff;
        border-radius: 4px;
        font-size: 12px;
    }

    .player-message {
        padding: 64px;
        text-align: center;
        color: #aaa;
    }
</style>
