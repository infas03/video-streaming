# Video Streaming Service

Repository: https://github.com/infas03/video-streaming.git

A private video streaming service built with Rust and SvelteKit. Videos are playable immediately after upload through HTTP range requests, while HLS transcoding happens in the background.

## How It Works

When a video is uploaded, it becomes available for streaming right away using the original file. A background worker picks up the file, transcodes it into HLS segments using FFmpeg, and uploads them to object storage. Once transcoding is done, the player seamlessly switches from the raw file to the optimized HLS stream without interrupting playback.

## Tech Stack

| Layer | Technology |
|---|---|
| Backend | Rust, Axum |
| Frontend | SvelteKit, HLS.js |
| Database | PostgreSQL |
| Queue | Redis |
| Storage | MinIO (dev), Cloudflare R2 (prod) |
| Transcoding | FFmpeg |

## Project Structure

```
server/
  src/
    bin/
      api_server.rs          API server entry point
      transcode_worker.rs    Worker entry point
    config.rs                Environment config
    state.rs                 Shared application state
    error.rs                 Error types
    db/
      mod.rs                 Database pool
      video.rs               Video and job queries
      migrations/            SQL migrations
    storage/
      mod.rs                 S3 operations (upload, presign, download)
    handlers/
      upload.rs              POST /api/upload
      video.rs               GET /api/videos/:token, GET /api/videos/:token/raw
      stream.rs              GET /api/videos/:token/manifest.m3u8, GET /api/videos/:token/status (SSE)
    worker/
      mod.rs                 Job loop and orchestration
      transcode.rs           FFmpeg and ffprobe wrapper
    models/
      video.rs               Video and TranscodeJob structs

client/
  src/
    routes/
      +page.svelte           Upload page
      v/[token]/+page.svelte Video player page
    lib/
      Upload.svelte          Upload component with progress
      VideoPlayer.svelte     HLS.js player with SSE upgrade

nginx/
  nginx.conf                 Reverse proxy config

docker-compose.yml           Full local environment
```

## Prerequisites

1. Rust (install from https://rustup.rs)
2. Node.js 22+
3. Docker and Docker Compose
4. FFmpeg (required by the transcode worker)

## Local Setup

Start the infrastructure containers:

```
docker compose up -d postgres redis minio minio-setup
```

Install the sqlx CLI and run migrations:

```
cargo install sqlx-cli --no-default-features --features postgres
cd server
sqlx migrate run --source src/db/migrations
```

Start the API server:

```
cd server
cargo run --bin api-server
```

Start the transcode worker in a separate terminal:

```
cd server
cargo run --bin transcode-worker
```

Install frontend dependencies and start the dev server in another terminal:

```
cd client
npm install
npm run dev
```

Open http://localhost:5173 in the browser.

## Environment Variables

These are configured in `server/.env` for local development:

```
DATABASE_URL=postgres://videouser:videopass@localhost:5433/videodb
REDIS_URL=redis://localhost:6379
S3_ENDPOINT=http://localhost:9000
S3_BUCKET=videos
S3_ACCESS_KEY=minioadmin
S3_SECRET_KEY=minioadmin
S3_REGION=us-east-1
SERVER_PORT=8080
MAX_UPLOAD_BYTES=1073741824
RUST_LOG=info
```

## API Endpoints

```
POST /api/upload                          Upload a video file (max 1 GB)
GET  /api/videos/:token                   Video metadata
GET  /api/videos/:token/raw               Redirect to raw file (presigned URL)
GET  /api/videos/:token/manifest.m3u8     Redirect to HLS manifest (presigned URL)
GET  /api/videos/:token/status            SSE stream for transcoding progress
```
