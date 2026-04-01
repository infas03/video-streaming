# Video Streaming Service — Architecture Document

## Overview

A minimal, private video streaming service designed around one core principle: **time-to-stream is the primary constraint**. The system makes videos available for streaming immediately after upload completes, without waiting for transcoding, by serving the original file via HTTP range requests. Adaptive HLS delivery is layered on top asynchronously.

---

## Stack

| Layer | Technology | Rationale |
|---|---|---|
| Backend | Rust / Axum | Low memory footprint, excellent async streaming, tower middleware |
| Frontend | SvelteKit + HLS.js | Minimal overhead, file-based routing, native ESM |
| Database | PostgreSQL (sqlx) | Reliable relational store; async driver; strong typing |
| Job queue | Redis (LPUSH / BRPOP) | Zero-dependency queue; reliable delivery; trivially scalable |
| Object storage | MinIO (dev) / Cloudflare R2 (prod) | S3-compatible API; R2 has zero egress fees — critical for video |
| Transcoding | FFmpeg via subprocess | Most capable open codec tool; streaming segment output |
| Dev orchestration | Docker Compose | Single command local environment |

---

## Core Architecture Decisions

### 1. Two-phase streaming: zero time-to-stream

The biggest risk in video services is the transcoding bottleneck. We eliminate it entirely by decoupling *availability* from *quality*:

**Phase 1 — Immediate (t=0 after upload)**  
After the upload completes, the API returns a share link. The video is instantly streamable via HTTP range requests on the raw file. Most modern browsers can play MP4, WebM, and MOV files natively this way. The player uses `<video src="presigned-url">` and streams byte-ranges directly from object storage.

**Phase 2 — Async (background, ~1–5 min)**  
A Rust worker picks up the transcoding job, runs FFmpeg to produce HLS segments (`.ts` + `.m3u8`), and uploads each segment to object storage as FFmpeg produces it. Once the manifest is ready, the API emits an SSE event. The player receives this, swaps its source to the HLS manifest, and switches seamlessly without interrupting playback. The user never notices.

This means: **upload 1 GB → share link is live → streaming works — all before a single byte of transcoding has happened.**

### 2. Upload: streaming multipart to object storage

The API uses Axum's `Multipart` extractor to stream the request body chunk-by-chunk directly into an S3 multipart upload (via `aws-sdk-s3`). There is no intermediate temp file. Memory usage for a 1 GB upload is bounded by the chunk buffer size (~5–10 MB), not the file size.

A `content-length` check at request start rejects files above 1 GB before any data is read. A MIME type check (from the `content-type` header + magic byte sniffing via `infer`) validates the video format.

### 3. Share token generation

When an upload completes, the API atomically:
1. Inserts a `videos` row with `status = 'ready'`
2. Generates a 12-character base62 token (`nanoid`)
3. Pushes a transcoding job to Redis
4. Returns `{ share_url: "https://host/v/{token}" }`

The token is the only routing key. No authentication is required. Anyone with the link can watch.

### 4. Object storage as the CDN

All video data — raw files and HLS segments — lives in object storage. The API never proxies segment bytes; it issues short-lived presigned URLs and redirects. Object storage handles all bandwidth.

In production, Cloudflare R2 + a `pub.*` bucket with a custom domain acts as a CDN. R2 charges zero egress, which is critical at scale: a 500 MB video watched 1,000 times would cost hundreds of dollars per month in S3 egress fees; on R2, it costs nothing beyond the storage ($0.015/GB/month).

### 5. HLS segment strategy

FFmpeg is invoked with a pipe-friendly output mode:
```
ffmpeg -i input.mp4 -c:v libx264 -preset fast -crf 23
       -c:a aac -b:a 128k
       -hls_time 4 -hls_playlist_type vod
       -hls_segment_filename "seg_%03d.ts"
       output.m3u8
```

A 4-second segment duration means the first segment is available for streaming ~4 seconds after transcoding begins. The worker uploads each `.ts` segment to object storage immediately using a `tokio::spawn` task, overlapping upload with continued transcoding. Playback can begin before FFmpeg finishes the entire file.

For the bonus "consistent playback regardless of file size": HLS achieves this by design — the player pre-buffers only the next few segments. A 10 GB file plays as smoothly as a 10 MB file because only 8–16 seconds of data is in the buffer at any time.

### 6. Horizontal scaling

The service is designed stateless by construction:

- **API servers** hold no in-memory state. Any number of instances behind a load balancer (Nginx/Caddy) work identically.
- **Transcode workers** pull from Redis with `BRPOP`. Adding more worker instances increases throughput linearly.
- **Object storage** (MinIO cluster or R2) is shared across all instances.
- **PostgreSQL** is the only shared mutable state. Connection pooling via `sqlx`'s built-in pool handles concurrent API instances; a PgBouncer sidecar can be added if connection counts become a bottleneck.

The two Rust binaries (`api-service`, `transcode-worker`) can be deployed independently on separate machines or in the same container for small deployments.

---

## Project Structure

```
video-streaming/
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 # Axum router + server startup
│       ├── config.rs               # Environment-based config (S3, DB, Redis)
│       ├── error.rs                # Unified AppError type → HTTP response
│       ├── db/
│       │   ├── mod.rs              # Connection pool init
│       │   ├── video.rs            # Video CRUD queries
│       │   └── migrations/         # sqlx migrate
│       ├── storage/
│       │   └── mod.rs              # S3 client wrapper (upload, presign, list)
│       ├── handlers/
│       │   ├── upload.rs           # POST /api/upload
│       │   ├── stream.rs           # GET /api/videos/:token/manifest.m3u8
│       │   │                       # GET /api/videos/:token/status (SSE)
│       │   └── video.rs            # GET /api/videos/:token (metadata)
│       ├── worker/
│       │   ├── mod.rs              # BRPOP loop, job dispatch
│       │   └── transcode.rs        # FFmpeg subprocess wrapper
│       └── models/
│           └── video.rs            # Video, TranscodeJob structs
├── frontend/
│   ├── package.json
│   └── src/
│       ├── routes/
│       │   ├── +page.svelte        # Upload page
│       │   └── v/[token]/
│       │       └── +page.svelte    # Video player page
│       └── lib/
│           ├── VideoPlayer.svelte  # HLS.js wrapper, SSE upgrade logic
│           └── Upload.svelte       # Chunked upload with progress
├── docker-compose.yml
└── docs/
    └── architecture.md             # This file
```

---

## Database Schema

```sql
CREATE TABLE videos (
  id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  token        VARCHAR(16) UNIQUE NOT NULL,
  filename     TEXT        NOT NULL,
  size_bytes   BIGINT      NOT NULL CHECK (size_bytes <= 1073741824),
  mime_type    TEXT        NOT NULL,
  storage_key  TEXT        NOT NULL,          -- e.g. "raw/abc123.mp4"
  status       TEXT        NOT NULL DEFAULT 'uploading',
  -- 'uploading' | 'ready' | 'transcoding' | 'done' | 'error'
  hls_ready    BOOLEAN     NOT NULL DEFAULT false,
  hls_key      TEXT,                          -- e.g. "hls/abc123/master.m3u8"
  duration_s   FLOAT,
  width        INT,
  height       INT,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE transcode_jobs (
  id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  video_id     UUID        NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
  status       TEXT        NOT NULL DEFAULT 'pending',
  -- 'pending' | 'running' | 'done' | 'error'
  error_msg    TEXT,
  attempts     INT         NOT NULL DEFAULT 0,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  started_at   TIMESTAMPTZ,
  done_at      TIMESTAMPTZ
);

CREATE INDEX ON videos(token);
CREATE INDEX ON transcode_jobs(status, created_at);
```

---

## API Surface

```
POST /api/upload
  Content-Type: multipart/form-data
  Body: file (≤1 GB, video/*)
  → 200 { video_id, token, share_url, watch_url }

GET  /api/videos/:token
  → 200 { id, token, status, hls_ready, duration_s, width, height, created_at }

GET  /api/videos/:token/manifest.m3u8
  → 302 redirect → presigned URL (object storage)
  → 404 if hls_ready = false (client falls back to range request mode)

GET  /api/videos/:token/status
  → text/event-stream (SSE)
  data: { status: "transcoding" }
  data: { status: "done", hls_ready: true }

GET  /v/:token
  → SvelteKit page (video player + share UI)
```

---

## Key Implementation Notes

### Upload handler (Rust)
```rust
// Pseudocode — actual impl uses aws-sdk-s3 CreateMultipartUpload
async fn upload(mut multipart: Multipart, State(state): State<AppState>)
  -> Result<Json<UploadResponse>, AppError>
{
  // 1. Validate file field, content-type, size header
  // 2. Initiate S3 multipart upload
  // 3. Stream chunks: while let Some(chunk) = field.chunk().await
  //      upload_part(chunk) → collect ETags
  // 4. Complete multipart upload
  // 5. Insert DB record (status = 'ready')
  // 6. LPUSH transcode_jobs {video_id}
  // 7. Return share link
}
```

### Transcode worker (Rust)
```rust
// Worker loop
loop {
  let job_id: String = redis.brpop("transcode_jobs", 0).await?;
  tokio::spawn(async move {
    // 1. Update job status → running
    // 2. Download raw file to /tmp/{video_id}.{ext}
    // 3. Run ffprobe → extract metadata (duration, resolution)
    // 4. Spawn ffmpeg process with HLS output to /tmp/{video_id}/
    // 5. Watch output dir: upload each new .ts segment to S3 as it appears
    // 6. Upload master.m3u8 last
    // 7. Update videos SET hls_ready=true, status='done'
    // 8. Cleanup /tmp files
  });
}
```

### Frontend player (Svelte)
```svelte
<!-- VideoPlayer.svelte — simplified -->
<script>
  import Hls from 'hls.js';
  let { token } = $props();

  onMount(() => {
    // Phase 1: start playing via range requests on raw file
    video.src = `/api/videos/${token}/raw`;  // presigned redirect

    // Phase 2: listen for HLS ready event
    const sse = new EventSource(`/api/videos/${token}/status`);
    sse.onmessage = ({ data }) => {
      const { hls_ready } = JSON.parse(data);
      if (hls_ready && Hls.isSupported()) {
        const hls = new Hls();
        hls.loadSource(`/api/videos/${token}/manifest.m3u8`);
        hls.attachMedia(video);  // seamless switch, preserves playhead
        sse.close();
      }
    };
  });
</script>
<video bind:this={video} controls playsinline />
```

---

## Deployment (Production)

```yaml
# Single VPS (e.g. Hetzner CX21, ~€4/mo) for small scale
services:
  api:
    image: video-api:latest
    replicas: 2                  # scale horizontally
    environment:
      DATABASE_URL: postgres://...
      REDIS_URL: redis://...
      S3_ENDPOINT: https://...r2.cloudflarestorage.com
      S3_BUCKET: videos
      MAX_UPLOAD_BYTES: 1073741824

  worker:
    image: video-worker:latest
    replicas: 2                  # scale independently
    environment: *api-env        # shares config

  nginx:
    image: nginx:alpine
    # load balances across api replicas
    # serves /v/* to SvelteKit static build
```

**Cost estimate (production, 100 videos/day at ~200 MB avg):**
- Hetzner CX21 × 2 (API + worker): ~€8/month
- Cloudflare R2: 100 × 200 MB × 30 days = 600 GB storage → $9/month
- R2 egress: $0 (zero egress billing)
- Total: **~$17/month** for a fully functional service

---

## Trade-offs Acknowledged

**Range request phase uses original encoding**  
The raw file phase plays whatever codec the user uploaded. On older browsers, this may fail for less common formats. The HLS upgrade resolves this universally. A future improvement: detect browser codec support and surface a "processing" UI state if the original format is unsupported.

**Single FFmpeg quality pass**  
For speed, we transcode to 720p only. A second pass adding 1080p and 480p renditions can be enqueued after the first completes, improving quality over time without affecting time-to-stream.

**Redis as job queue**  
Redis with LPUSH/BRPOP is simple and reliable but lacks built-in retry with backoff. A production hardening step would add job attempt counting (already in schema) with exponential backoff re-queuing on failure, or migrate to a proper queue like `apalis` (Rust-native).

**No CDN cache invalidation**  
HLS segments are immutable by design (content-addressed keys). The manifest is served via API with `Cache-Control: no-store` until `hls_ready=true`, then flipped to `Cache-Control: public, max-age=31536000`. No invalidation needed.
