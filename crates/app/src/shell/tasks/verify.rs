use iced::Task;
use iced::futures::SinkExt;
use ssh_core::scanner::Device;
use tokio::sync::mpsc;

use crate::message::Message;

pub(super) fn spawn_verify_task(
    devices: Vec<Device>,
    username: String,
    password: Option<String>,
    session_id: u64,
) -> Task<Message> {
    let stream = iced::stream::channel::<Message>(100, async move |mut output| {
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
        let worker_devices = devices.clone();
        let worker_username = username.clone();
        let worker_password = password.clone();
        let worker = tokio::spawn(async move {
            ssh_core::ssh::auth::verify_devices(
                &worker_devices,
                &worker_username,
                worker_password.as_deref(),
                ssh_core::ssh::auth::VERIFY_CONCURRENCY,
                move |ip, status| {
                    let _ = progress_tx.send((ip.to_owned(), status));
                },
            )
            .await
        });

        while let Some((ip, status)) = progress_rx.recv().await {
            if output
                .send(Message::VerifyResult {
                    session_id,
                    ip,
                    status,
                })
                .await
                .is_err()
            {
                return;
            }
        }

        let _ = worker.await;
        let _ = output.send(Message::VerifyComplete { session_id }).await;
    });

    Task::run(stream, |message| message)
}
