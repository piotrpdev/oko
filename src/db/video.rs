use serde::{Deserialize, Serialize};
use sqlx::{Result, SqlitePool};
use time::OffsetDateTime;

use crate::db::VideoCameraView;

use super::Model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub video_id: i64,
    pub camera_id: Option<i64>,
    pub file_path: String,
    pub start_time: OffsetDateTime,
    pub end_time: Option<OffsetDateTime>,
    pub file_size: Option<i64>,
}

pub struct Default {
    pub video_id: i64,
    pub end_time: Option<OffsetDateTime>,
}

impl Default {
    pub fn start_time() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

impl Model for Video {
    type Default = Default;
    const DEFAULT: Default = Default {
        video_id: -1,
        end_time: None,
    };

    async fn create_using_self(&mut self, pool: &SqlitePool) -> Result<()> {
        let result = sqlx::query!(
            r#"
            INSERT INTO videos (camera_id, file_path, start_time, end_time, file_size)
            VALUES (?, ?, ?, ?, ?)
            RETURNING video_id
            "#,
            self.camera_id,
            self.file_path,
            self.start_time,
            self.end_time,
            self.file_size
        )
        .fetch_one(pool)
        .await?;

        self.video_id = result.video_id;

        Ok(())
    }

    async fn get_using_id(pool: &SqlitePool, id: i64) -> Result<Self> {
        sqlx::query_as!(
            Video,
            r#"
            SELECT video_id, camera_id, file_path, start_time, end_time, file_size
            FROM videos WHERE video_id = ?
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }

    async fn update_using_self(&self, pool: &SqlitePool) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE videos
            SET camera_id = ?, end_time = ?, file_size = ?
            WHERE video_id = ?
            RETURNING video_id
            "#,
            self.camera_id,
            self.end_time,
            self.file_size,
            self.video_id
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }

    async fn delete_using_id(pool: &SqlitePool, id: i64) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE
            FROM videos
            WHERE video_id = ?
            RETURNING video_id
            "#,
            id
        )
        .fetch_one(pool)
        .await?;

        Ok(())
    }
}

impl Video {
    pub async fn list_for_camera(
        db: &sqlx::Pool<sqlx::Sqlite>,
        camera_id: i64,
    ) -> Result<Vec<VideoCameraView>> {
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

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras", "videos")))]
    async fn create(pool: SqlitePool) -> Result<()> {
        let mut video = Video {
            video_id: Video::DEFAULT.video_id,
            camera_id: Some(1),
            file_path: "/path/to/video.mp4".to_string(),
            start_time: OffsetDateTime::now_utc(),
            end_time: Video::DEFAULT.end_time,
            file_size: Some(1024),
        };

        video.create_using_self(&pool).await?;

        assert_eq!(video.video_id, 3);

        let returned_video = Video::get_using_id(&pool, 3).await?;

        assert_eq!(returned_video.camera_id, video.camera_id);
        assert_eq!(returned_video.file_path, video.file_path);
        assert_eq!(returned_video.start_time, video.start_time);
        assert_eq!(returned_video.end_time, video.end_time);
        assert_eq!(returned_video.file_size, video.file_size);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras", "videos")))]
    async fn get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
        let video_id = 1;
        let returned_video = Video::get_using_id(&pool, video_id).await?;

        assert_eq!(returned_video.video_id, video_id);
        assert_eq!(returned_video.camera_id, Some(1));
        assert_eq!(
            returned_video.file_path,
            "/home/piotrpdev/oko/scripts/1.mp4"
        );
        assert_eq!(
            returned_video.start_time,
            OffsetDateTime::from_unix_timestamp(1_729_479_512)?
        );
        assert_eq!(returned_video.file_size, Some(6_762_403));

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras", "videos")))]
    async fn update(pool: SqlitePool) -> Result<()> {
        let old_video = Video::get_using_id(&pool, 1).await?;

        let updated_video = Video {
            video_id: old_video.video_id,
            camera_id: Some(1),
            file_path: old_video.file_path,
            start_time: old_video.start_time,
            end_time: Some(OffsetDateTime::now_utc()),
            file_size: Some(2048),
        };

        let updated = updated_video.update_using_self(&pool).await;
        assert!(updated.is_ok());

        let returned_video = Video::get_using_id(&pool, old_video.video_id).await?;
        assert_eq!(returned_video.camera_id, updated_video.camera_id);
        assert_eq!(returned_video.file_path, updated_video.file_path);
        assert_eq!(returned_video.start_time, updated_video.start_time);
        assert_eq!(returned_video.end_time, updated_video.end_time);
        assert_eq!(returned_video.file_size, updated_video.file_size);

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras", "videos")))]
    async fn delete(pool: SqlitePool) -> Result<()> {
        let video_id = 1;
        let deleted = Video::delete_using_id(&pool, video_id).await;
        assert!(deleted.is_ok());

        let returned_video_result = Video::get_using_id(&pool, video_id).await;
        assert!(returned_video_result.is_err());

        let impossible_deleted = Video::delete_using_id(&pool, video_id).await;

        assert!(impossible_deleted.is_err());

        Ok(())
    }

    #[sqlx::test(fixtures(path = "../../fixtures", scripts("cameras", "videos")))]
    async fn list_for_camera(pool: SqlitePool) -> Result<()> {
        let camera_id = 1;
        let returned_videos = Video::list_for_camera(&pool, camera_id).await?;

        assert_eq!(returned_videos.len(), 1);
        assert_eq!(returned_videos.first().unwrap().video_id, 1);
        assert_eq!(returned_videos.first().unwrap().camera_id, Some(1));
        assert_eq!(returned_videos.first().unwrap().camera_name, "Front Door");
        assert_eq!(
            returned_videos.first().unwrap().file_path,
            "/home/piotrpdev/oko/scripts/1.mp4"
        );
        assert_eq!(returned_videos.first().unwrap().file_size, Some(6_762_403));

        Ok(())
    }
}
