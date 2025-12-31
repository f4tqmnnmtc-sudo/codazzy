CREATE TABLE IF NOT EXISTS prediction_cache (
    id VARCHAR(128) PRIMARY KEY,
    device_id VARCHAR(64) NOT NULL,
    device_name VARCHAR(128) NOT NULL,
    metric_name VARCHAR(64) NOT NULL,
    display_name VARCHAR(128),
    current_value DOUBLE PRECISION NOT NULL,
    predicted_value DOUBLE PRECISION NOT NULL,
    threshold_value DOUBLE PRECISION NOT NULL,
    threshold_type VARCHAR(16) NOT NULL,
    predicted_at VARCHAR(64) NOT NULL,
    confidence INTEGER NOT NULL,
    trend VARCHAR(16) NOT NULL,
    hours_until DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_prediction_cache_device ON prediction_cache(device_id);
CREATE INDEX IF NOT EXISTS idx_prediction_cache_hours ON prediction_cache(hours_until);

