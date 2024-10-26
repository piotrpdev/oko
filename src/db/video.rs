use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

use crate::db::VideoCameraView;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub video_id: i64,
    pub camera_id: Option<i64>,
    pub file_path: String,
    pub start_time: OffsetDateTime,
    pub end_time: Option<OffsetDateTime>,
    pub file_size: Option<i64>,
}

#[allow(dead_code)]
impl Video {
    pub async fn create(
        pool: &SqlitePool,
        camera_id: i64,
        file_path: &str,
        start_time: OffsetDateTime,
        file_size: Option<i64>,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO videos (camera_id, file_path, start_time, file_size)
            VALUES (?, ?, ?, ?)
            RETURNING video_id
            "#,
            camera_id,
            file_path,
            start_time,
            file_size
        )
        .fetch_one(pool)
        .await?;

        Ok(result.video_id)
    }

    pub async fn get(pool: &SqlitePool, video_id: i64) -> Result<Video> {
        sqlx::query_as!(
            Video,
            r#"
            SELECT video_id, camera_id, file_path, start_time, end_time, file_size
            FROM videos WHERE video_id = ?
            "#,
            video_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        video_id: i64,
        end_time: Option<OffsetDateTime>,
        file_size: Option<i64>,
    ) -> Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE videos
            SET end_time = ?, file_size = ?
            WHERE video_id = ?
            "#,
            end_time,
            file_size,
            video_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn delete(pool: &SqlitePool, video_id: i64) -> Result<bool> {
        let rows_affected = sqlx::query!("DELETE FROM videos WHERE video_id = ?", video_id)
            .execute(pool)
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn list_for_camera(
        db: &sqlx::Pool<sqlx::Sqlite>,
        camera_id: i64,
    ) -> sqlx::Result<Vec<VideoCameraView>> {
        sqlx::query_as!(
            VideoCameraView,
            r#"
            SELECT v.video_id, v.camera_id, c.name as camera_name, v.file_path, v.file_size
            FROM videos v
            JOIN cameras c ON v.camera_id = c.camera_id
            WHERE c.camera_id = ?
            "#,
            camera_id
        )
        .fetch_all(db)
        .await
    }
}