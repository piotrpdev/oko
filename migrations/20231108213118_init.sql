CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cameras (
    camera_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    ip_address TEXT,
    last_connected TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE IF NOT EXISTS camera_permissions (
    permission_id INTEGER PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER,
    user_id INTEGER,
    can_view BOOLEAN NOT NULL DEFAULT true,
    can_control BOOLEAN NOT NULL DEFAULT false,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id)
);

CREATE TABLE IF NOT EXISTS videos (
    video_id INTEGER PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER,
    file_path TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    file_size INTEGER,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
);

CREATE TABLE IF NOT EXISTS camera_settings (
    setting_id INTEGER PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER,
    flashlight_enabled BOOLEAN NOT NULL DEFAULT false,
    resolution TEXT NOT NULL,
    framerate INTEGER NOT NULL,
    last_modified TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified_by INTEGER NOT NULL,
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id),
    FOREIGN KEY (modified_by) REFERENCES users(user_id)
);

INSERT INTO users (user_id, username, password_hash, created_at) VALUES
    (1, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw', '2024-10-21 17:01:23'),
    (2, 'piotrpdev', '$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw', '2024-10-21 17:02:18'),
    (3, 'joedaly', '$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw', '2024-10-21 17:12:32');

INSERT INTO cameras (camera_id, name, ip_address, last_connected) VALUES
    (1, 'Front Door', '192.168.0.169', '2024-10-20 17:56:18'),
    (2, 'Kitchen', '192.168.0.172', '2024-10-20 17:57:22');

INSERT INTO camera_permissions (camera_id, user_id, can_view, can_control) VALUES
    (1, 1, true, true),
    (2, 1, true, true),
    (1, 2, true, true),
    (2, 2, true, true),
    (1, 3, true, false),
    (2, 3, false, false);

INSERT INTO videos (camera_id, file_path, start_time, end_time, file_size) VALUES
    (1, '/home/piotrpdev/oko/scripts/1.mp4', '2024-10-21 02:58:32', '2024-10-21 03:01:12', 6762403),
    (2, '/home/piotrpdev/oko/scripts/2.mp4', '2024-10-21 02:57:56', '2024-10-21 03:03:23', 6905856);

INSERT INTO camera_settings (camera_id, flashlight_enabled, resolution, framerate, last_modified, modified_by) VALUES
    (1, false, '800x600', 5, '2024-10-21 17:02:33', 1),
    (2, false, '800x600', 5, '2024-10-21 17:02:25', 1);
