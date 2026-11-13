//! Serialized application boundary. The desktop owns this service behind one
//! mutex; renderer lifetime never owns tracking or persistence.
use catch_core::{growth, recurrence, resources, todos, tracking};
use catch_storage::{Disk, GrowthStore, TodosStore, TrackingStore};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "group", content = "command", rename_all = "kebab-case")]
pub enum UserCommand {
    Tracking(tracking::Command),
    Todos(todos::Command),
    Growth(growth::Command),
    Resources(resources::Command),
    Recurrence(RecurrenceRequest),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRequest {
    pub command: recurrence::Command,
    pub expected_todos_revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub request_id: String,
    pub expected_revision: u64,
    pub operation: UserCommand,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub tracking: tracking::Document,
    pub todos: todos::Document,
    pub growth: growth::Document,
    pub recurrences: recurrence::Document,
    pub tracking_notice: Option<String>,
    pub storage_blocked: bool,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Validation,
    Conflict,
    StorageFailure,
}
#[derive(Clone, Debug, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}
impl Error {
    fn validation(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Validation,
            message: message.into(),
        }
    }
    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Conflict,
            message: message.into(),
        }
    }
    fn storage(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::StorageFailure,
            message: message.into(),
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;
/// Only the trusted host provides this context, never an IPC request argument.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub now: i64,
    pub local_day: &'a str,
    pub new_id: &'a str,
}
struct Cached {
    request_id: String,
    fingerprint: Vec<u8>,
    result: Snapshot,
}
pub struct Service {
    tracking: TrackingStore,
    todos: TodosStore,
    growth: GrowthStore,
    cache: VecDeque<Cached>,
}
impl Service {
    pub fn open(disk: Arc<dyn Disk>, context: Context<'_>) -> Result<Self> {
        todos::calendar_date(context.local_day).map_err(Error::validation)?;
        // Validate/recover todos first: a broken archive must not silently
        // become an empty successful snapshot.
        let growth = GrowthStore::open(disk.clone()).map_err(Error::storage)?;
        let mut todos = TodosStore::open(disk.clone()).map_err(Error::storage)?;
        let tracking =
            TrackingStore::open(disk, context.now, context.new_id).map_err(Error::storage)?;
        todos
            .refresh_recurrences(context.local_day)
            .map_err(Error::storage)?;
        Ok(Self {
            tracking,
            todos,
            growth,
            cache: VecDeque::new(),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tracking: self.tracking.snapshot().clone(),
            todos: self.todos.snapshot().clone(),
            growth: self.growth.snapshot().clone(),
            recurrences: self.todos.recurrences().clone(),
            tracking_notice: self.tracking.notice.clone(),
            storage_blocked: self.tracking.is_blocked()
                || self.todos.is_blocked()
                || self.growth.is_blocked(),
        }
    }
    pub fn execute(&mut self, request: Request, context: Context<'_>) -> Result<Snapshot> {
        let fingerprint =
            serde_json::to_vec(&request).map_err(|error| Error::validation(error.to_string()))?;
        if request.request_id.is_empty()
            || request.request_id.encode_utf16().count() > 100
            || fingerprint.len() > 100_000
        {
            return Err(Error::validation("请求标识无效或操作内容过大"));
        }
        if let Some(previous) = self
            .cache
            .iter()
            .find(|item| item.request_id == request.request_id)
        {
            if previous.fingerprint != fingerprint {
                return Err(Error::conflict("重复请求参数不一致"));
            }
            return Ok(previous.result.clone());
        }
        todos::calendar_date(context.local_day).map_err(Error::validation)?;
        match request.operation.clone() {
            UserCommand::Recurrence(operation) => {
                if request.expected_revision != self.todos.recurrences().revision
                    || operation.expected_todos_revision != self.todos.snapshot().revision
                {
                    return Err(Error::conflict("周期规则或待办已变化，请核对后重试"));
                }
                self.todos
                    .recurrences()
                    .command(
                        &operation.command,
                        self.todos.snapshot(),
                        request.expected_revision,
                        context.local_day,
                        context.new_id,
                    )
                    .map_err(Error::validation)?;
                self.todos
                    .recurrence_command(
                        &operation.command,
                        request.expected_revision,
                        context.local_day,
                        context.new_id,
                    )
                    .map_err(Error::storage)?;
            }
            UserCommand::Resources(command) => {
                let current = self.resources(command.owner())?;
                let next = command
                    .apply(current, context.new_id)
                    .map_err(Error::validation)?;
                match command.owner() {
                    resources::Owner::Matter { id } => {
                        if request.expected_revision != self.tracking.snapshot().revision {
                            return Err(Error::conflict("事项已变化，请核对后重试"));
                        }
                        self.tracking
                            .command(
                                &tracking::Command::SetResources {
                                    id: id.clone(),
                                    resources: next,
                                },
                                request.expected_revision,
                                tracking::Context {
                                    now: context.now,
                                    new_id: context.new_id,
                                },
                            )
                            .map_err(Error::storage)?;
                    }
                    resources::Owner::Todo { id } => {
                        if request.expected_revision != self.todos.snapshot().revision {
                            return Err(Error::conflict("待办已变化，请核对后重试"));
                        }
                        self.todos
                            .command(
                                &todos::Command::SetResources {
                                    id: id.clone(),
                                    resources: next,
                                },
                                request.expected_revision,
                                todos::Context {
                                    now: context.now,
                                    local_day: context.local_day,
                                    new_id: context.new_id,
                                },
                            )
                            .map_err(Error::storage)?;
                    }
                }
            }
            UserCommand::Growth(command) => {
                if request.expected_revision != self.growth.snapshot().revision {
                    return Err(Error::conflict("今日植物或目标已变化，请核对后重试"));
                }
                self.growth
                    .snapshot()
                    .apply(
                        &command,
                        request.expected_revision,
                        growth::Context {
                            now: context.now,
                            local_day: context.local_day,
                        },
                    )
                    .map_err(Error::validation)?;
                self.growth
                    .command(
                        &command,
                        request.expected_revision,
                        growth::Context {
                            now: context.now,
                            local_day: context.local_day,
                        },
                    )
                    .map_err(Error::storage)?;
            }
            UserCommand::Tracking(mut command) => {
                if request.expected_revision != self.tracking.snapshot().revision {
                    return Err(Error::conflict("投入记录已变化，请核对后重试"));
                }
                if matches!(command, tracking::Command::Checkpoint)
                    || matches!(&command,tracking::Command::Pause{reason,cutoff} if reason!="manual"||cutoff.is_some())
                {
                    return Err(Error::validation("系统计时操作不能从界面调用"));
                }
                if let tracking::Command::Start {
                    ref mut name,
                    todo_id: Some(ref todo_id),
                    ref mut resources,
                } = command
                {
                    let todo = self
                        .todos
                        .snapshot()
                        .todos
                        .iter()
                        .find(|todo| todo.id == *todo_id && !todo.done && todo.deleted_at.is_none())
                        .ok_or_else(|| Error::validation("待办已完成或已删除"))?;
                    *name = todo.text.clone();
                    *resources = todo.resources.clone();
                }
                let clock = tracking::Context {
                    now: context.now,
                    new_id: context.new_id,
                };
                // The same pure operation validates business errors before I/O,
                // allowing the public error code to distinguish a storage fault.
                self.tracking
                    .snapshot()
                    .apply(&command, request.expected_revision, clock)
                    .map_err(Error::validation)?;
                self.tracking
                    .command(
                        &command,
                        request.expected_revision,
                        tracking::Context {
                            now: context.now,
                            new_id: context.new_id,
                        },
                    )
                    .map_err(Error::storage)?;
            }
            UserCommand::Todos(command) => {
                if request.expected_revision != self.todos.snapshot().revision {
                    return Err(Error::conflict("待办已变化，请核对后重试；草稿已保留"));
                }
                let candidate = self
                    .todos
                    .snapshot()
                    .apply(
                        &command,
                        request.expected_revision,
                        todos::Context {
                            now: context.now,
                            local_day: context.local_day,
                            new_id: context.new_id,
                        },
                    )
                    .map_err(Error::validation)?;
                self.todos
                    .recurrences()
                    .after_todo_change(self.todos.snapshot(), &candidate, context.local_day)
                    .map_err(Error::validation)?;
                self.todos
                    .command(
                        &command,
                        request.expected_revision,
                        todos::Context {
                            now: context.now,
                            local_day: context.local_day,
                            new_id: context.new_id,
                        },
                    )
                    .map_err(Error::storage)?;
            }
        }
        let result = self.snapshot();
        self.cache.push_back(Cached {
            request_id: request.request_id,
            fingerprint,
            result: result.clone(),
        });
        if self.cache.len() > 512 {
            self.cache.pop_front();
        }
        Ok(result)
    }
    pub fn interrupt(&mut self, now: i64, reason: &str) -> Result<Snapshot> {
        self.tracking
            .interrupt(now, reason)
            .map_err(Error::storage)?;
        Ok(self.snapshot())
    }
    fn resources(&self, owner: &resources::Owner) -> Result<&[resources::Resource]> {
        match owner {
            resources::Owner::Todo { id } => self
                .todos
                .snapshot()
                .todos
                .iter()
                .find(|row| row.id == *id && row.deleted_at.is_none())
                .map(|row| row.resources.as_slice())
                .ok_or_else(|| Error::validation("待办已不存在或已删除")),
            resources::Owner::Matter { id } => self
                .tracking
                .snapshot()
                .activities
                .iter()
                .find(|row| row.id == *id)
                .map(|row| row.resources.as_slice())
                .ok_or_else(|| Error::validation("事项已不存在")),
        }
    }
    /// The desktop opener resolves an existing association by ID. No arbitrary
    /// renderer-supplied URL/path is passed directly to ShellExecute.
    pub fn resource_for_open(
        &self,
        owner: &resources::Owner,
        id: &str,
    ) -> Result<resources::Resource> {
        let resource = self
            .resources(owner)?
            .iter()
            .find(|resource| resource.id == id)
            .ok_or_else(|| Error::validation("资料关联已不存在"))?;
        resource.validate().map_err(Error::validation)?;
        Ok(resource.clone())
    }
    pub fn checkpoint(&mut self, now: i64) -> Result<Snapshot> {
        if let Err(error) = self.tracking.checkpoint(now) {
            let _ = self.tracking.interrupt(now, "save-error");
            return Err(Error::storage(error));
        }
        Ok(self.snapshot())
    }
    pub fn refresh_recurrences(&mut self, today: &str) -> Result<bool> {
        self.todos
            .refresh_recurrences(today)
            .map_err(Error::storage)
    }
    /// Called by the trusted timer/host, not exposed as a user command. Return
    /// true only after the first achievement was durably recorded.
    pub fn observe_growth(&mut self, bounds: growth::DayBounds<'_>, now: i64) -> Result<bool> {
        let invested =
            growth::invested(self.tracking.snapshot(), &bounds, now).map_err(Error::validation)?;
        let before = self
            .growth
            .snapshot()
            .days
            .get(bounds.day)
            .is_some_and(|day| day.celebrated_at.is_some());
        self.growth
            .observe(
                growth::Context {
                    now,
                    local_day: bounds.day,
                },
                invested,
            )
            .map_err(Error::storage)?;
        Ok(!before
            && self
                .growth
                .snapshot()
                .days
                .get(bounds.day)
                .is_some_and(|day| day.celebrated_at.is_some()))
    }
    pub fn growth_view(&self, bounds: growth::DayBounds<'_>, now: i64) -> Result<growth::View> {
        let invested =
            growth::invested(self.tracking.snapshot(), &bounds, now).map_err(Error::validation)?;
        self.growth
            .snapshot()
            .view(bounds.day, invested)
            .map_err(Error::validation)
    }
}
