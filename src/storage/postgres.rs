use std::{marker::PhantomData, fmt::Debug};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions, types::Uuid, FromRow};
use ulid::Ulid;
use indoc::indoc;
use chrono::{Utc, DateTime};

pub use sqlx::postgres::PgConnectOptions;

use crate::{
    error::{JobRunError, StorageError},
    JobType, JobTypeMarker, StorageProvider, Job,
};

use super::{JobMetadata, JobState, JobInfo};

#[derive(Clone)]
pub struct PostgresStorageProvider<J: JobTypeMarker + ?Sized> {
    pool: Pool<Postgres>,
    _phantom_data: PhantomData<J>,
}

impl<J: JobTypeMarker + ?Sized> PostgresStorageProvider<J> {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            _phantom_data: PhantomData,
        }
    }

    pub async fn from_options(options: PgConnectOptions) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        Ok(Self::new(pool))
    }
}

#[async_trait]
impl<J: JobTypeMarker + ?Sized> StorageProvider<J> for PostgresStorageProvider<J>
where
    Box<J>: DeserializeOwned,
{
    async fn pull(&mut self) -> Result<JobInfo<J>, StorageError> {
        let now = chrono::Utc::now();
        let result = sqlx::query_as::<_, DbJob>(indoc!{"
            UPDATE job_queue
            SET state = $2, started = $1
            WHERE id IN (
                SELECT id
                FROM job_queue
                WHERE state = $3
                ORDER BY created
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *
        "})
            .bind(now)
            .bind(JobState::Running)
            .bind(JobState::NotStarted)
            .fetch_one(&self.pool).await?;

        Ok(result.into_job_info()?)
    }

    async fn push(&mut self, job: &J) -> Result<JobMetadata, StorageError> {
        let uid: Uuid = Ulid::new().into();
        let job_type = J::job_type();
        let data = serde_json::to_value(&job)?;
        let created = Utc::now();

        let result = sqlx::query_as::<_, DbJob>(indoc!{"
                INSERT INTO job_queue
                    (uid, type, data, created)
                VALUES
                    ($1, $2, $3, $4)
                RETURNING *
            "})
            .bind(uid).bind(job_type).bind(data).bind(created)
            .fetch_one(&self.pool).await?;

        Ok(JobMetadata {
            uid: Ulid::from(result.uid),
            state: result.state,
            result: result.result.map(serde_json::from_value).transpose()?,
        })
    }

    async fn set_job_result(
        &mut self,
        uid: Ulid,
        job_result: Result<(), JobRunError>,
    ) -> Result<JobMetadata, StorageError> {
        let job_state = if job_result.is_ok() {
            JobState::Completed
        } else {
            JobState::Failed
        };
        let job_result = job_result.err().map(serde_json::to_value).transpose()?;

        let result: DbJob = sqlx::query_as(indoc!{"
                UPDATE job_queue
                SET result = $1, state = $2
                WHERE uid = $3
                RETURNING *
            "})
            .bind(job_result)
            .bind(job_state)
            .bind(Uuid::from(uid))
            .fetch_one(&self.pool).await?;

        Ok(result.into_job_metadata()?)
    }

    async fn get_job(&self, job_id: Ulid) -> Result<JobMetadata, StorageError> {
        let result = sqlx::query_as::<_, DbJob>(indoc!{"
            SELECT *
            FROM job_queue
            WHERE uid = $1
        "})
            .bind(&Uuid::from(job_id))
            .fetch_one(&self.pool).await?;

        Ok(result.into_job_metadata()?)
    }
}

#[derive(sqlx::FromRow)]
pub struct DbJob {
    id: i32,
    uid: Uuid,
    #[sqlx(rename = "type")]
    job_type: String,
    data: Value,
    result: Option<Value>,
    state: JobState,
    created: DateTime<Utc>,
    started: Option<DateTime<Utc>>,
    completed: Option<DateTime<Utc>>,
}

impl DbJob {
    pub fn into_job_info<J: JobTypeMarker + ?Sized>(self) -> Result<JobInfo<J>, serde_json::Error>
    where
        Box<J>: DeserializeOwned,
    {
        let metadata = JobMetadata {
            uid: Ulid::from(self.uid),
            state: self.state,
            result: self.result.map(serde_json::from_value).transpose()?,
        };

        let job: Box<J> = serde_json::from_value(self.data)?;

        Ok(JobInfo { metadata, job })
    }

    pub fn into_job_metadata(self) -> Result<JobMetadata, serde_json::Error> {
        Ok(JobMetadata {
            uid: Ulid::from(self.uid),
            state: self.state,
            result: self.result.map(serde_json::from_value).transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use async_trait::async_trait;
    use sqlx::{Pool, Postgres};

    use super::{PgConnectOptions, PostgresStorageProvider};
    use crate::{job, job_type, Job, StorageProvider, storage::JobState};

    #[job_type]
    struct MockJobType {}

    #[job(MockJobType)]
    #[derive(PartialEq)]
    struct MockJob {
        msg: String,
    }

    #[async_trait]
    impl Job for MockJob {
        type JobTypeData = MockJobType;
        async fn run(&self, _: &Self::JobTypeData) {}
    }

    #[job(MockJobType)]
    #[derive(PartialEq)]
    struct MockJob2 {
        msg2: String,
    }

    #[async_trait]
    impl Job for MockJob2 {
        type JobTypeData = MockJobType;
        async fn run(&self, _: &Self::JobTypeData) {}
    }

    #[sqlx::test]
    async fn test_push_pull(conn: Pool<Postgres>) {
        let mut storage = PostgresStorageProvider::<dyn MockJobTypeMarker>::new(conn);

        let job1 = MockJob { msg: "a".to_string() };
        let job2 = MockJob2 { msg2: "b".to_string() };

        storage.push(&job1).await.unwrap();
        storage.push(&job2).await.unwrap();

        assert_eq!(*storage.pull().await.unwrap().job.into_any().downcast::<MockJob>().unwrap(), job1);
        assert_eq!(*storage.pull().await.unwrap().job.into_any().downcast::<MockJob2>().unwrap(), job2);
    }

    #[sqlx::test]
    async fn test_set_job_status(conn: Pool<Postgres>) {
        let mut storage = PostgresStorageProvider::<dyn MockJobTypeMarker>::new(conn);

        let job = MockJob { msg: "a".to_string() };

        let job_meta = storage.push(&job).await.unwrap();
        assert_eq!(*storage.pull().await.unwrap().job.into_any().downcast::<MockJob>().unwrap(), job);

        storage.set_job_result(job_meta.uid, Ok(())).await.unwrap();

        let job_meta = storage.get_job(job_meta.uid).await.unwrap();
        assert_eq!(job_meta.state, JobState::Completed);
    }
}
