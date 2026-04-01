CREATE TABLE videos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token VARCHAR(16) UNIQUE NOT NULL,
    filename TEXT NOT NULL,
    size_bytes BIGINT NOT NULL CHECK (size_bytes <= 1073741824),
    mime_type TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'uploading',
    hls_ready BOOLEAN NOT NULL DEFAULT false,
    hls_key TEXT,
    duration_seconds FLOAT,
    width INT,
    height INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_videos_token ON videos(token);
