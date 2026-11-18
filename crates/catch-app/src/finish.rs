//! Finishing a matter, its optional todo and its durable replay receipt.
use crate::{Context, Error, FinishRequest, Request, Result, Service};
use catch_core::{todos, tracking};

impl Service {
    pub(super) fn finish(
        &mut self,
        request: &Request,
        operation: FinishRequest,
        fingerprint: &[u8],
        context: Context<'_>,
    ) -> Result<()> {
        if request.expected_revision != self.tracking.snapshot().revision
            || operation.expected_todos_revision != self.todos.snapshot().revision
        {
            return Err(Error::conflict("投入记录或待办已变化，请核对后重试"));
        }
        let tracking = self
            .tracking
            .snapshot()
            .finish(
                &operation.activity_id,
                operation.session_id.as_deref(),
                operation.progress.as_deref(),
                request.expected_revision,
                tracking::Context {
                    now: context.now,
                    new_id: context.new_id,
                },
            )
            .map_err(Error::validation)?;
        let mut todos = self.todos.snapshot().clone();
        if operation.complete_todo {
            let matter = tracking
                .activities
                .iter()
                .find(|row| row.id == operation.activity_id)
                .unwrap();
            let id = matter
                .todo_id
                .as_deref()
                .ok_or_else(|| Error::validation("事项没有关联待办"))?;
            if !todos
                .todos
                .iter()
                .any(|row| row.id == id && !row.done && row.deleted_at.is_none())
            {
                return Err(Error::validation("关联待办已完成、删除或不存在"));
            }
            todos = todos
                .apply(
                    &todos::Command::Complete {
                        id: id.into(),
                        done: true,
                    },
                    operation.expected_todos_revision,
                    todos::Context {
                        now: context.now,
                        local_day: context.local_day,
                        new_id: context.new_id,
                    },
                )
                .map_err(Error::validation)?;
        }
        let rules = self
            .todos
            .recurrences()
            .after_todo_change(self.todos.snapshot(), &todos, context.local_day)
            .map_err(Error::validation)?;
        let mut response = self.snapshot();
        response.tracking = tracking.clone();
        response.todos = todos.clone();
        response.recurrences = rules.clone();
        let receipt = catch_storage::finish::Receipt::new(
            request.request_id.clone(),
            fingerprint,
            serde_json::to_string(&response).map_err(|error| Error::storage(error.to_string()))?,
        );
        catch_storage::finish::commit(
            &mut self.tracking,
            &mut self.todos,
            catch_storage::finish::Candidate {
                tracking,
                todos,
                rules,
            },
            receipt,
            &mut self.receipts,
        )
        .map_err(Error::storage)?;
        Ok(())
    }
}
