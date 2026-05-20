CREATE TABLE IF NOT EXISTS awsem_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS s3_notification_configs (
    bucket_name TEXT PRIMARY KEY,
    config_xml  TEXT NOT NULL,
    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cognito_user_pools (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    arn        TEXT NOT NULL UNIQUE,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cognito_clients (
    id            TEXT PRIMARY KEY,
    pool_id       TEXT NOT NULL REFERENCES cognito_user_pools(id),
    client_name   TEXT NOT NULL,
    client_id     TEXT NOT NULL UNIQUE,
    client_secret TEXT NOT NULL,
    config_json   TEXT NOT NULL DEFAULT '{}',
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cognito_users (
    id              TEXT PRIMARY KEY,
    pool_id         TEXT NOT NULL REFERENCES cognito_user_pools(id),
    username        TEXT NOT NULL,
    password_hash   TEXT NOT NULL,
    email           TEXT,
    phone           TEXT,
    status          TEXT NOT NULL DEFAULT 'UNCONFIRMED',
    attributes_json TEXT NOT NULL DEFAULT '{}',
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(pool_id, username)
);

CREATE TABLE IF NOT EXISTS secrets_secrets (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    arn         TEXT NOT NULL UNIQUE,
    description TEXT,
    kms_key_id  TEXT,
    tags_json   TEXT DEFAULT '[]',
    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_changed TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at   TIMESTAMP
);

CREATE TABLE IF NOT EXISTS secrets_versions (
    id                TEXT PRIMARY KEY,
    secret_id         TEXT NOT NULL REFERENCES secrets_secrets(id),
    version_id        TEXT NOT NULL,
    value             TEXT NOT NULL,
    staging_labels_json TEXT NOT NULL DEFAULT '[]',
    created_at        TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS emr_virtual_clusters (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    arn         TEXT NOT NULL UNIQUE,
    namespace   TEXT NOT NULL,
    eks_cluster TEXT NOT NULL DEFAULT '',
    state       TEXT NOT NULL DEFAULT 'RUNNING',
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS emr_job_runs (
    id                 TEXT PRIMARY KEY,
    name               TEXT NOT NULL,
    arn                TEXT NOT NULL UNIQUE,
    virtual_cluster_id TEXT NOT NULL REFERENCES emr_virtual_clusters(id),
    job_driver_json    TEXT NOT NULL,
    state              TEXT NOT NULL DEFAULT 'PENDING',
    kubernetes_job_name TEXT,
    output_prefix      TEXT,
    created_at         TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    finished_at        TIMESTAMP
);

CREATE TABLE IF NOT EXISTS lambda_functions (
    name          TEXT PRIMARY KEY,
    arn           TEXT NOT NULL UNIQUE,
    runtime       TEXT NOT NULL,
    handler       TEXT NOT NULL,
    image         TEXT,
    code_zip      BLOB,
    role          TEXT NOT NULL,
    timeout       INTEGER DEFAULT 3,
    memory_size   INTEGER DEFAULT 128,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS lambda_event_source_mappings (
    id               TEXT PRIMARY KEY,
    function_arn     TEXT NOT NULL REFERENCES lambda_functions(arn),
    event_source_arn TEXT NOT NULL,
    enabled          INTEGER DEFAULT 1,
    batch_size       INTEGER DEFAULT 10,
    created_at       TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
