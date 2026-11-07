use std::future::Future;
use std::time::Duration;

use median_core::{Event, TaskReport, TaskState};
use median_engine::Input;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::sleep;
use tracing::{error, warn};

/// What to do when a long-lived task ends.
#[derive(Debug, Clone, Copy)]
pub enum Restart {
    /// Start it again after this delay.
    After(Duration),
    /// Leave it stopped.
    Never,
}

/// Spawns `task` under `name`, reports each state (running, restarting, stopped) to the engine
/// and restarts it per `restart`; stops for good once the engine input is closed.
pub fn spawn<F, Fut>(name: &'static str, inputs: Sender<Input>, restart: Restart, mut task: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        loop {
            if !report(&inputs, name, TaskState::Running) {
                return;
            }

            let reason = match tauri::async_runtime::spawn(task()).await {
                Ok(Ok(())) => "the task finished".to_string(),
                Ok(Err(reason)) => reason,
                Err(e) => format!("the task panicked: {e}"),
            };

            let Restart::After(delay) = restart else {
                error!(task = name, %reason, "task stopped, not restarted");
                report(&inputs, name, TaskState::Stopped { reason });
                return;
            };

            warn!(task = name, restart_in_s = delay.as_secs(), %reason, "task stopped, restarting");
            if !report(&inputs, name, TaskState::Restarting { reason }) {
                return;
            }
            sleep(delay).await;
        }
    });
}

/// Sends a task state to the engine without waiting; returns `false` once the engine input is
/// closed. A full queue drops the report and still returns `true`.
fn report(inputs: &Sender<Input>, name: &'static str, state: TaskState) -> bool {
    let report = TaskReport {
        name: name.to_string(),
        state,
    };
    match inputs.try_send(Input::Command(Event::TaskReported(report))) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) => {
            warn!(
                task = name,
                "task state not reported, engine input queue full"
            );
            true
        }
        Err(TrySendError::Closed(_)) => false,
    }
}
