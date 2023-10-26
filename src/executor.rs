use crate::error::ExecutionError;
use crate::error::StorageError;
use crate::JobTypeMarker;

use super::Job;
use super::StorageProvider;

use tokio::select;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tokio::{sync::broadcast, task};

#[derive(Clone, Debug)]
enum BroadcastMessage {
    Shutdown,
}

pub struct Executor<J: JobTypeMarker + ?Sized> {
    job_type_data: J::JobTypeData,
    storage_provider: Box<dyn StorageProvider<J>>,
}

impl<J: JobTypeMarker + ?Sized + 'static> Executor<J> {
    pub fn new<S: StorageProvider<J> + 'static>(
        storage_provider: S, job_type_data: J::JobTypeData,
    ) -> Self {
        Self { job_type_data, storage_provider: Box::new(storage_provider) }
    }

    pub fn start(self) -> RunningExecutor {
        let (sender, receiver) = broadcast::channel(1);
        let (notifier_sender, notifier_receiver) = broadcast::channel(10);

        let run_notifier_sender = notifier_sender.clone();

        let join = task::spawn(async move {
            select! {
                _ = self.run(run_notifier_sender) => {}
                _ = manage_signals(receiver) => {}
            }
        });

        RunningExecutor {
            task_handle: join,
            broadcast_channel: sender,
            notifier: (notifier_sender, notifier_receiver),
            waited_for: 0,
        }
    }

    async fn run(mut self, notifier: broadcast::Sender<u32>) {
        let mut i = 0;
        loop {
            let result = self.storage_provider.pull().await;
            if let Err(StorageError::Database(sqlx::Error::RowNotFound)) = result {
                time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            let job_info = result.expect("Failed to fetch job");
            Job::run(&*job_info.job, &self.job_type_data).await;
            self.storage_provider
                .set_job_result(job_info.metadata.uid, Ok(()))
                .await
                .expect("Failed to set job result");

            i += 1;
            let _ = notifier.send(i);
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
    notifier: (broadcast::Sender<u32>, broadcast::Receiver<u32>),
    waited_for: u32,
}

impl RunningExecutor {
    pub async fn stop(self) -> Result<(), ExecutionError> {
        self.broadcast_channel
            .send(BroadcastMessage::Shutdown)
            .map_err(|x| ExecutionError::SignalingError(Box::new(x)))?;
        self.task_handle.await.map_err(ExecutionError::JoinError)?;
        Ok(())
    }

    async fn wait_for_forever(&mut self, number_of_messages: u32) -> Result<(), ()> {
        loop {
            match self.notifier.1.recv().await {
                Ok(value) if value >= self.waited_for + number_of_messages => {
                    self.waited_for += number_of_messages;
                    return Ok(());
                }
                Ok(_) => time::sleep(Duration::from_millis(10)).await,
                Err(_) => return Err(()),
            }
        }
    }

    pub async fn wait_for(&mut self, number_of_messages: u32, timeout: Duration) -> Result<(), ()> {
        match time::timeout(timeout, self.wait_for_forever(number_of_messages)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(())) => Err(()),
            Err(_) => Err(()),
        }
    }
}
