-- =========================================================================
-- Schema: Equalizer FD CRAFT Studio Database (SQLite / Embedded Engine)
-- File: replica/schema.sql
-- =========================================================================

PRAGMA foreign_keys = ON;

-- 1. Audio Endpoints Table
CREATE TABLE IF NOT EXISTS audio_endpoints (
    id TEXT PRIMARY KEY,                       -- Windows Endpoint GUID (e.g. '{0.0.0.00000000}.{...}')
    name TEXT NOT NULL,                        -- Friendly device name
    adapter_name TEXT NOT NULL,                -- Hardware adapter ID
    endpoint_type TEXT NOT NULL CHECK(endpoint_type IN ('playback', 'capture')),
    is_default BOOLEAN NOT NULL DEFAULT 0,
    apo_installed BOOLEAN NOT NULL DEFAULT 0,
    hook_mode TEXT NOT NULL DEFAULT 'SFX_EFX' CHECK(hook_mode IN ('LFX_GFX', 'SFX_MFX', 'SFX_EFX')),
    preserve_original_apo BOOLEAN NOT NULL DEFAULT 1,
    stage_flags INTEGER NOT NULL DEFAULT 3,    -- Bitmask: 1=PreMix, 2=PostMix
    last_synced_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE INDEX IF NOT EXISTS idx_endpoints_type ON audio_endpoints(endpoint_type);
CREATE INDEX IF NOT EXISTS idx_endpoints_installed ON audio_endpoints(apo_installed);

-- 2. Profiles Table (Presets: Gaming, Studio, Movie, Speech)
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,                       -- UUID
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 0,
    hotkey TEXT,                               -- Optional keyboard shortcut
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE INDEX IF NOT EXISTS idx_profiles_active ON profiles(is_active);

-- 3. Device to Profile Bindings
CREATE TABLE IF NOT EXISTS device_profile_bindings (
    endpoint_id TEXT NOT NULL REFERENCES audio_endpoints(id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    PRIMARY KEY (endpoint_id, profile_id)
);

-- 4. Filter Pipeline Definitions
CREATE TABLE IF NOT EXISTS pipelines (
    id TEXT PRIMARY KEY,                       -- UUID
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name TEXT NOT NULL DEFAULT 'Main Pipeline',
    sample_rate INTEGER NOT NULL DEFAULT 48000,
    channel_count INTEGER NOT NULL DEFAULT 2,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

CREATE INDEX IF NOT EXISTS idx_pipelines_profile ON pipelines(profile_id);

-- 5. Ordered Pipeline Processing Blocks
CREATE TABLE IF NOT EXISTS pipeline_blocks (
    id TEXT PRIMARY KEY,                       -- UUID
    pipeline_id TEXT NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,                 -- Execution order in audio thread
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    block_type TEXT NOT NULL CHECK(block_type IN (
        'preamp', 'biquad', 'graphic_eq', 'delay', 'channel_matrix', 'vst_host', 'include_file'
    )),
    scoped_channels TEXT NOT NULL DEFAULT 'all', -- Comma-separated or 'all'
    title TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    CONSTRAINT uq_pipeline_position UNIQUE(pipeline_id, position)
);

CREATE INDEX IF NOT EXISTS idx_blocks_pipeline ON pipeline_blocks(pipeline_id, position);

-- 6. Preamp Parameters
CREATE TABLE IF NOT EXISTS block_preamp (
    block_id TEXT PRIMARY KEY REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    gain_db REAL NOT NULL DEFAULT 0.0 CHECK(gain_db BETWEEN -60.0 AND 30.0)
);

-- 7. Biquad Filter Parameters
CREATE TABLE IF NOT EXISTS block_biquad (
    block_id TEXT PRIMARY KEY REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    filter_type TEXT NOT NULL CHECK(filter_type IN ('PK', 'LS', 'HS', 'LP', 'HP', 'BP', 'NO', 'AP')),
    frequency_hz REAL NOT NULL CHECK(frequency_hz BETWEEN 10.0 AND 24000.0),
    gain_db REAL NOT NULL DEFAULT 0.0 CHECK(gain_db BETWEEN -60.0 AND 30.0),
    q_factor REAL NOT NULL DEFAULT 1.4142 CHECK(q_factor BETWEEN 0.05 AND 100.0),
    bandwidth_octaves REAL
);

CREATE INDEX IF NOT EXISTS idx_biquad_fc ON block_biquad(frequency_hz);

-- 8. Graphic EQ Points (Variable or ISO 15/31)
CREATE TABLE IF NOT EXISTS block_graphic_eq (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id TEXT NOT NULL REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    frequency_hz REAL NOT NULL,
    gain_db REAL NOT NULL DEFAULT 0.0,
    CONSTRAINT uq_eq_point UNIQUE(block_id, frequency_hz)
);

CREATE INDEX IF NOT EXISTS idx_graphic_eq_block ON block_graphic_eq(block_id, frequency_hz);

-- 9. Delay Parameters
CREATE TABLE IF NOT EXISTS block_delay (
    block_id TEXT PRIMARY KEY REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    delay_ms REAL NOT NULL DEFAULT 0.0 CHECK(delay_ms BETWEEN 0.0 AND 5000.0)
);

-- 10. Channel Matrix Routing Rules
CREATE TABLE IF NOT EXISTS block_channel_matrix (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id TEXT NOT NULL REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    target_channel TEXT NOT NULL,              -- e.g. 'L', 'R', 'SUB'
    formula TEXT NOT NULL                      -- e.g. '0.7*L + 0.3*R'
);

-- 11. VST Plugin Host Parameters
CREATE TABLE IF NOT EXISTS block_vst (
    block_id TEXT PRIMARY KEY REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    vst_version TEXT NOT NULL DEFAULT 'vst2' CHECK(vst_version IN ('vst2', 'vst3')),
    plugin_path TEXT NOT NULL,
    serialized_state BLOB                      -- Plugin chunk state data
);

-- 12. Include Directives
CREATE TABLE IF NOT EXISTS block_include (
    block_id TEXT PRIMARY KEY REFERENCES pipeline_blocks(id) ON DELETE CASCADE,
    include_path TEXT NOT NULL
);
