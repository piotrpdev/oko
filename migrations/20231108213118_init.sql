CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE CHECK(LENGTH(username) <= 256),
    password_hash TEXT NOT NULL CHECK(LENGTH(password_hash) <= 256),
    created_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS cameras (
    camera_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL CHECK(LENGTH(name) <= 256),
    ip_address TEXT CHECK(ip_address IS NULL OR LENGTH(ip_address) <= 64),
    last_connected TIMESTAMP,
    is_active BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS camera_permissions (
    permission_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    can_view BOOLEAN NOT NULL,
    can_control BOOLEAN NOT NULL,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS videos (
    video_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER,
    file_path TEXT NOT NULL CHECK(LENGTH(file_path) <= 4096),
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    file_size INTEGER,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS camera_settings (
    setting_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER NOT NULL,
    flashlight_enabled BOOLEAN NOT NULL,
    resolution TEXT NOT NULL CHECK(LENGTH(resolution) <= 20),
    framerate INTEGER NOT NULL CHECK(framerate <= 256),
    last_modified TIMESTAMP NOT NULL,
    modified_by INTEGER,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE,
    FOREIGN KEY (modified_by) REFERENCES users(user_id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_camera_permissions_camera ON camera_permissions (camera_id);
CREATE INDEX IF NOT EXISTS idx_camera_permissions_user ON camera_permissions (user_id);

CREATE INDEX IF NOT EXISTS idx_videos_camera ON videos (camera_id);

CREATE INDEX IF NOT EXISTS idx_camera_settings_camera ON camera_settings (camera_id);

INSERT INTO users (user_id, username, password_hash, created_at) VALUES
    (1, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw', '2024-10-21 17:01:23');
