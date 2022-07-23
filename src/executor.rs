use super::Job;
use super::StorageProvider;

use tokio::select;
use tokio::task::JoinHandle;
use tokio::{sync::broadcast, task};

#[derive(Clone, Debug)]
enum BroadcastMessage {
    Shutdown,
}

pub struct Executor<J: Job + ?Sized> {
    job_type_data: J::JobTypeData,
    storage_provider: Box<dyn StorageProvider<J>>,
}

impl<J: Job + ?Sized + 'static> Executor<J> {
    pub fn new(
        storage_provider: Box<dyn StorageProvider<J>>,
        job_type_data: J::JobTypeData,
    ) -> Self {
        Self {
            job_type_data,
            storage_provider,
        }
    }

    pub async fn start(self) -> RunningExecutor {
        let (sender, receiver) = broadcast::channel(1);

        let join = task::spawn(async move {
            select! {
                _ = self.run() => {}
                _ = manage_signals(receiver) => {}
            }
        });

        RunningExecutor {
            task_handle: join,
            broadcast_channel: sender,
        }
    }

    async fn run(mut self) {
        while let Ok(job) = self.storage_provider.get_job().await {
            Job::run(&*job, &self.job_type_data).await;
        }
    }
}

async fn manage_signals(mut receiver: broadcast::Receiver<BroadcastMessage>) {
    loop {
        match receiver.recv().await {
            Ok(BroadcastMessage::Shutdown) => break,
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => {}
        }
    }
}

pub struct RunningExecutor {
    task_handle: JoinHandle<()>,
    broadcast_channel: broadcast::Sender<BroadcastMessage>,
}

impl RunningExecutor {
    pub async fn stop(self) {
        self.broadcast_channel
            .send(BroadcastMessage::Shutdown)
            .unwrap();
        self.task_handle.await.unwrap();
    }
}
