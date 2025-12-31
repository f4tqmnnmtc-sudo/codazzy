-- Migración inicial: tablas principales de Codazzy Gateway

CREATE TABLE IF NOT EXISTS agent_connections (
    id SERIAL PRIMARY KEY,
    node_id VARCHAR(255) UNIQUE NOT NULL,
    ssh_hostname VARCHAR(255) NOT NULL,
    ssh_port INTEGER DEFAULT 22,
    ssh_username VARCHAR(255),
    config_path VARCHAR(512),
    agent_path VARCHAR(512),
    location VARCHAR(255),
    environment VARCHAR(100) DEFAULT 'production',
    tags TEXT[],
    os_type VARCHAR(50),
    installation_method VARCHAR(100) DEFAULT 'remote_ssh',
    job_id VARCHAR(255),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_connected_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS server_metadata (
    node_id VARCHAR(255) PRIMARY KEY,
    display_name VARCHAR(255),
    description TEXT,
    location VARCHAR(255),
    environment VARCHAR(100),
    owner VARCHAR(255),
    contact_email VARCHAR(255),
    tags TEXT[],
    custom_fields JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS detected_services (
    id SERIAL PRIMARY KEY,
    node_id VARCHAR(255) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    status VARCHAR(50) DEFAULT 'unknown',
    process_count INTEGER DEFAULT 0,
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    exe_path VARCHAR(512),
    UNIQUE(node_id, service_name)
);

CREATE TABLE IF NOT EXISTS detected_processes (
    id SERIAL PRIMARY KEY,
    node_id VARCHAR(255) NOT NULL,
    pid INTEGER NOT NULL,
    process_name VARCHAR(255) NOT NULL,
    exe_path VARCHAR(512),
    cpu_usage DOUBLE PRECISION DEFAULT 0,
    memory_bytes BIGINT DEFAULT 0,
    status VARCHAR(50) DEFAULT 'unknown',
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(node_id, pid)
);

CREATE TABLE IF NOT EXISTS alert_thresholds (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL,
    metric_name VARCHAR(255) NOT NULL,
    warning_threshold DOUBLE PRECISION,
    critical_threshold DOUBLE PRECISION,
    comparison VARCHAR(10) DEFAULT 'gt',
    duration_seconds INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(device_id, metric_name)
);

CREATE TABLE IF NOT EXISTS alerts (
    id VARCHAR(255) PRIMARY KEY,
    node_id VARCHAR(255) NOT NULL,
    metric_name VARCHAR(255) NOT NULL,
    severity VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'active',
    value DOUBLE PRECISION NOT NULL,
    threshold_warning DOUBLE PRECISION,
    threshold_critical DOUBLE PRECISION,
    message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS discovered_devices (
    id VARCHAR(255) PRIMARY KEY,
    ip_address VARCHAR(45) NOT NULL,
    mac_address VARCHAR(17),
    hostname VARCHAR(255),
    status VARCHAR(50) DEFAULT 'discovered',
    device_type VARCHAR(100) DEFAULT 'unknown',
    vendor VARCHAR(255),
    os VARCHAR(255),
    open_ports INTEGER[] DEFAULT '{}',
    available_protocols TEXT[] DEFAULT '{}',
    response_time_ms DOUBLE PRECISION DEFAULT 0,
    source VARCHAR(50) DEFAULT 'network_scan',
    container_id VARCHAR(64),
    container_name VARCHAR(255),
    container_image VARCHAR(512),
    container_status VARCHAR(50),
    location VARCHAR(255),
    environment VARCHAR(100),
    tags TEXT[] DEFAULT '{}',
    notes TEXT,
    custom_fields JSONB DEFAULT '{}',
    first_discovered_at TIMESTAMPTZ DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS server_processes (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL,
    process_name VARCHAR(255) NOT NULL,
    pid INTEGER,
    cpu_usage REAL,
    memory_bytes BIGINT,
    memory_percent REAL,
    status VARCHAR(50),
    exe_path VARCHAR(512),
    command TEXT,
    collected_at TIMESTAMPTZ NOT NULL,
    UNIQUE(device_id, pid, collected_at)
);

CREATE TABLE IF NOT EXISTS server_services (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    status VARCHAR(50),
    process_count INTEGER,
    total_cpu REAL,
    total_memory BIGINT,
    collected_at TIMESTAMPTZ NOT NULL,
    UNIQUE(device_id, service_name, collected_at)
);

CREATE TABLE IF NOT EXISTS device_thresholds (
    id SERIAL PRIMARY KEY,
    device_id VARCHAR(255) NOT NULL,
    device_name VARCHAR(255),
    device_type VARCHAR(100),
    metric_name VARCHAR(255) NOT NULL,
    metric_display_name VARCHAR(255),
    metric_unit VARCHAR(50),
    warning_threshold DOUBLE PRECISION,
    critical_threshold DOUBLE PRECISION,
    comparison VARCHAR(10) DEFAULT 'gt',
    priority VARCHAR(20) DEFAULT 'medium',
    ai_reasoning TEXT,
    ai_model_used VARCHAR(100),
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(device_id, metric_name)
);

