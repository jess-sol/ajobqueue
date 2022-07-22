use super::Job;
use super::StorageProvider;

pub struct Executor<J: Job + ?Sized> {
    job_type_data: J::JobTypeData,
    storage_provider: Box<dyn StorageProvider<Job=J>>,
}


impl<J: Job + ?Sized> Executor<J> {
    pub fn new(storage_provider: Box<dyn StorageProvider<Job=J>>, job_type_data: J::JobTypeData) -> Self {
        Self { job_type_data, storage_provider }
    }

    pub async fn start(&mut self) {
        let job: Box<J> = self.storage_provider.get_job().await.unwrap();
        Job::run(&*job, &self.job_type_data).await;
    }
}
