use oko::{Camera, CameraPermission, CameraSetting, User, Video};
use sqlx::{Result, SqlitePool};

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "cameras", "camera_permissions", "videos", "camera_settings")))]
async fn camera_on_delete(pool: SqlitePool) -> Result<()> {
    let camera_id = 1;
    let permission_id = 1;
    let video_id = 1;
    let setting_id = 1;

    let permission = CameraPermission::get(&pool, permission_id).await?;
    assert_eq!(permission.camera_id, camera_id);

    let video = Video::get(&pool, video_id).await?;
    assert_eq!(video.camera_id, Some(camera_id));

    let setting = CameraSetting::get(&pool, setting_id).await?;
    assert_eq!(setting.camera_id, camera_id);

    let deleted = Camera::delete(&pool, camera_id).await?;
    assert!(deleted);

    let permission = CameraPermission::get(&pool, permission_id).await;
    assert!(permission.is_err());

    let video = Video::get(&pool, video_id).await?;
    assert_eq!(video.camera_id, None);

    let setting = CameraSetting::get(&pool, setting_id).await;
    assert!(setting.is_err());

    Ok(())
}

#[sqlx::test(fixtures(path = "../fixtures", scripts("users", "cameras", "camera_permissions", "camera_settings")))]
async fn user_on_delete(pool: SqlitePool) -> Result<()> {
    let user_id = 2;
    let camera_id = 2;
    let permission_id = 4;
    let setting_id = 2;

    let permission = CameraPermission::get(&pool, permission_id).await?;
    assert_eq!(permission.camera_id, camera_id);
    assert_eq!(permission.user_id, user_id);

    let setting = CameraSetting::get(&pool, setting_id).await?;
    assert_eq!(setting.camera_id, camera_id);
    assert_eq!(setting.modified_by, Some(user_id));

    let deleted = User::delete(&pool, user_id).await?;
    assert!(deleted);

    let permission = CameraPermission::get(&pool, permission_id).await;
    assert!(permission.is_err());

    let setting = CameraSetting::get(&pool, setting_id).await?;
    assert_eq!(setting.modified_by, None);

    Ok(())
}