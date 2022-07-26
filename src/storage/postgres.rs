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

use super::{JobMetadata, JobState};

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
    async fn pull(&mut self) -> Result<Box<J>, StorageError> {
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
            .fetch_one(&self.pool).await
            .map_err(|x| StorageError::FetchFailure(Box::new(x)))?;

        Ok(serde_json::from_value(result.data)?)
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
            .fetch_one(&self.pool).await
            .map_err(|x| StorageError::CreateFailure(Box::new(x)))?;

        Ok(JobMetadata {
            uid: Ulid::from(result.uid),
            state: result.state,
            result: result.result.map(serde_json::from_value).transpose()?,
        })
    }

    async fn set_job_result(
        &mut self,
        _result: Result<(), JobRunError>,
    ) -> Result<(), StorageError> {
        unimplemented!()
    }
}

#[derive(sqlx::FromRow)]
struct DbJob {
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::{PgConnectOptions, PostgresStorageProvider};
    use crate::{job, job_type, Job, StorageProvider};

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

    fn create_db_config() -> PgConnectOptions {
        PgConnectOptions::new()
            .host("127.0.0.1").database("job_queue")
            .username("postgres").password("postgres")
    }

    #[tokio::test]
    async fn test_push_pull() {
        let mut storage = PostgresStorageProvider::<dyn MockJobTypeMarker>::from_options(create_db_config()).await.unwrap();

        let job1 = MockJob { msg: "a".to_string() };
        let job2 = MockJob2 { msg2: "b".to_string() };

        storage.push(&job1).await.unwrap();
        storage.push(&job2).await.unwrap();

        assert_eq!(*storage.pull().await.unwrap().into_any().downcast::<MockJob>().unwrap(), job1);
        assert_eq!(*storage.pull().await.unwrap().into_any().downcast::<MockJob2>().unwrap(), job2);
    }
}
