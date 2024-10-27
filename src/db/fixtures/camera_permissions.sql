INSERT INTO camera_permissions (permission_id, camera_id, user_id, can_view, can_control) VALUES
    (1, 1, 1, true, true),
    (2, 2, 1, true, true),
    (3, 1, 2, true, true),
    (4, 2, 2, true, true),
    (5, 1, 3, true, false),
    (6, 2, 3, false, false);
