/// State of a background task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Running,
    /// The task ended and will be started again.
    Restarting {
        reason: String,
    },
    /// The task ended and will not be started again.
    Stopped {
        reason: String,
    },
}

/// Latest state a background task reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskReport {
    /// Task name, e.g. `collector`.
    pub name: String,
    pub state: TaskState,
}

/// Latest report of each background task, in the order the tasks first reported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tasks(Vec<TaskReport>);

impl Tasks {
    /// Latest report of every task.
    pub fn reports(&self) -> &[TaskReport] {
        &self.0
    }
}

/// Returns `tasks` with `report` replacing the earlier report of the same name, or appended when
/// the task reports for the first time.
pub(crate) fn update(tasks: &Tasks, report: TaskReport) -> Tasks {
    let mut next = tasks.clone();
    match next.0.iter_mut().find(|held| held.name == report.name) {
        Some(held) => *held = report,
        None => next.0.push(report),
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(name: &str, state: TaskState) -> TaskReport {
        TaskReport {
            name: name.into(),
            state,
        }
    }

    #[test]
    fn a_task_reporting_for_the_first_time_is_added() {
        let tasks = update(&Tasks::default(), report("collector", TaskState::Running));

        assert_eq!(tasks.reports(), [report("collector", TaskState::Running)]);
    }

    #[test]
    fn a_task_reporting_again_replaces_what_it_said() {
        let tasks = update(&Tasks::default(), report("collector", TaskState::Running));
        let died = TaskState::Restarting {
            reason: "the socket closed".into(),
        };

        let tasks = update(&tasks, report("collector", died.clone()));

        assert_eq!(tasks.reports(), [report("collector", died)]);
    }

    #[test]
    fn tasks_keep_the_order_they_first_reported_in() {
        let tasks = update(&Tasks::default(), report("worldstate", TaskState::Running));
        let tasks = update(&tasks, report("collector", TaskState::Running));

        let tasks = update(
            &tasks,
            report(
                "worldstate",
                TaskState::Stopped {
                    reason: "gave up".into(),
                },
            ),
        );

        let names: Vec<&str> = tasks
            .reports()
            .iter()
            .map(|report| report.name.as_str())
            .collect();
        assert_eq!(names, ["worldstate", "collector"]);
    }
}
