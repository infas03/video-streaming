<script>
    let file = $state(null);
    let uploading = $state(false);
    let progress = $state(0);
    let error = $state('');
    let result = $state(null);

    function handleFileSelect(event) {
        const selected = event.target.files[0];
        if (!selected) return;

        const maxSize = 1024 * 1024 * 1024;
        if (selected.size > maxSize) {
            error = 'File exceeds 1 GB limit';
            return;
        }

        if (!selected.type.startsWith('video/')) {
            error = 'Only video files are allowed';
            return;
        }

        error = '';
        result = null;
        file = selected;
    }

    async function uploadFile() {
        if (!file) return;

        uploading = true;
        progress = 0;
        error = '';
        result = null;

        const formData = new FormData();
        formData.append('file', file);

        const xhr = new XMLHttpRequest();

        xhr.upload.addEventListener('progress', (event) => {
            if (event.lengthComputable) {
                progress = Math.round((event.loaded / event.total) * 100);
            }
        });

        xhr.addEventListener('load', () => {
            uploading = false;
            if (xhr.status === 200) {
                result = JSON.parse(xhr.responseText);
            } else {
                try {
                    const body = JSON.parse(xhr.responseText);
                    error = body.error || 'Upload failed';
                } catch {
                    error = 'Upload failed';
                }
            }
        });

        xhr.addEventListener('error', () => {
            uploading = false;
            error = 'Network error during upload';
        });

        xhr.open('POST', '/api/upload');
        xhr.send(formData);
    }

    function formatSize(bytes) {
        if (bytes >= 1024 * 1024 * 1024) return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
        if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
        if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
        return bytes + ' B';
    }

    function reset() {
        file = null;
        uploading = false;
        progress = 0;
        error = '';
        result = null;
    }
</script>

<div class="upload-container">
    {#if result}
        <div class="upload-success">
            <h3>Upload complete</h3>
            <p>Your video is ready to watch.</p>
            <a href={result.share_url} class="watch-link">Watch Video</a>
            <button onclick={reset} class="btn-secondary">Upload Another</button>
        </div>
    {:else}
        <div class="upload-form">
            <label class="file-input-label">
                <input
                    type="file"
                    accept="video/*"
                    onchange={handleFileSelect}
                    disabled={uploading}
                />
                {#if file}
                    <span>{file.name} ({formatSize(file.size)})</span>
                {:else}
                    <span>Choose a video file</span>
                {/if}
            </label>

            {#if error}
                <p class="error">{error}</p>
            {/if}

            {#if uploading}
                <div class="progress-bar">
                    <div class="progress-fill" style="width: {progress}%"></div>
                </div>
                <p class="progress-text">{progress}%</p>
            {/if}

            <button
                onclick={uploadFile}
                disabled={!file || uploading}
                class="btn-primary"
            >
                {uploading ? 'Uploading...' : 'Upload'}
            </button>
        </div>
    {/if}
</div>

<style>
    .upload-container {
        max-width: 480px;
        margin: 0 auto;
    }

    .upload-form, .upload-success {
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .file-input-label {
        display: block;
        padding: 32px;
        border: 2px dashed #555;
        border-radius: 8px;
        text-align: center;
        cursor: pointer;
        transition: border-color 0.2s;
    }

    .file-input-label:hover {
        border-color: #aaa;
    }

    .file-input-label input {
        display: none;
    }

    .file-input-label span {
        color: #ccc;
        font-size: 14px;
    }

    .progress-bar {
        height: 8px;
        background: #333;
        border-radius: 4px;
        overflow: hidden;
    }

    .progress-fill {
        height: 100%;
        background: #4f8eff;
        transition: width 0.2s;
    }

    .progress-text {
        text-align: center;
        font-size: 14px;
        color: #aaa;
        margin: 0;
    }

    .btn-primary {
        padding: 12px;
        background: #4f8eff;
        color: white;
        border: none;
        border-radius: 6px;
        font-size: 16px;
        cursor: pointer;
    }

    .btn-primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .btn-secondary {
        padding: 10px;
        background: transparent;
        color: #4f8eff;
        border: 1px solid #4f8eff;
        border-radius: 6px;
        font-size: 14px;
        cursor: pointer;
    }

    .watch-link {
        display: block;
        padding: 12px;
        background: #4f8eff;
        color: white;
        border-radius: 6px;
        text-align: center;
        text-decoration: none;
        font-size: 16px;
    }

    .error {
        color: #ff4f4f;
        font-size: 14px;
        margin: 0;
    }

    .upload-success h3 {
        margin: 0;
    }
</style>
