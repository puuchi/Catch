<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import Modal from './Modal.vue';
import SessionTimeFields from './SessionTimeFields.vue';
import DatePicker from './DatePicker.vue';
import { busy, dispatch, state, ui } from '../store';
import { human, resizedSegments } from '../display';
import { closeEditor, sessionEditorBusy } from '../session-editor';
const saving = ref(false);
const sessionForm = computed(() => ui.editor?.kind === 'record' || ui.editor?.kind === 'session');
async function requestClose() {
  if (saving.value) return;
  error.value = '';
  try {
    await closeEditor();
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}
const error = ref('');
let compositionEnded = -Infinity;
let composing = false;
function compositionEnd() {
  composing = false;
  compositionEnded = performance.now();
}
function todoKey(event: KeyboardEvent) {
  if (
    ui.editor?.kind !== 'todo' ||
    event.isComposing ||
    event.keyCode === 229 ||
    performance.now() - compositionEnded < 80
  )
    return;
  const target = event.target as HTMLElement;
  if (event.key !== 'Enter' || event.repeat) return;
  if (
    (target.tagName === 'INPUT' && (target as HTMLInputElement).type === 'text') ||
    (target.tagName === 'TEXTAREA' && (event.ctrlKey || event.metaKey))
  ) {
    event.preventDefault();
    void save();
  }
}
const title = computed(() =>
  ui.editor?.kind === 'todo'
    ? '编辑待办'
    : ui.editor?.kind === 'matter'
      ? '编辑事项'
      : ui.editor?.kind === 'session'
        ? `编辑本次 · ${state.value?.tracking.activities.find((m) => m.id === ui.editor?.activityId)?.name || ui.editor.name}`
        : '补录投入',
);
watch(
  () => ui.editor,
  () => {
    error.value = '';
    composing = false;
    compositionEnded = -Infinity;
  },
  { flush: 'sync' },
);
const recurring = computed(() => {
  const row = state.value?.todos.todos.find((t) => t.id === ui.editor?.id);
  return state.value?.recurrences.templates.find(
    (t) => t.id === row?.recurrence?.templateId && !t.stopped,
  );
});
const previewDuration = computed(() => {
  const draft = ui.editor;
  if (!draft?.start || !draft.end) return '';
  try {
    const from = new Date(draft.start).toISOString(),
      to = new Date(draft.end).toISOString();
    return human(
      draft.original
        ? resizedSegments(draft.original, from, to).reduce(
            (n, s) => n + Date.parse(s.end!) - Date.parse(s.start),
            0,
          )
        : Math.max(0, Date.parse(to) - Date.parse(from)),
    );
  } catch {
    return '';
  }
});
async function save() {
  if (saving.value || sessionEditorBusy.value || busy.value) return;
  const draft = ui.editor;
  if (!draft) return;
  if (draft.kind === 'todo' && (composing || performance.now() - compositionEnded < 80)) return;
  error.value = '';
  saving.value = true;
  try {
    if (['todo', 'matter'].includes(draft.kind) && !draft.name.trim()) throw Error('先写下名称');
    if (draft.kind === 'todo') {
      const fields = {
        text: draft.name,
        notes: draft.notes,
        urgent: draft.urgent,
        deadline: draft.deadline || null,
      };
      if (draft.scope === 'future' && recurring.value)
        await dispatch('recurrence', 'update', {
          id: recurring.value.id,
          fields,
          resources: recurring.value.resources,
          schedule: recurring.value.schedule,
        });
      else await dispatch('todos', 'edit', { id: draft.id, ...fields });
    } else if (draft.kind === 'matter')
      await dispatch('tracking', 'update-matter', {
        id: draft.id,
        name: draft.name,
        status: draft.status,
      });
    else {
      if (!draft.start || !draft.end) throw Error('填写开始和结束时间');
      const startTime = new Date(draft.start).toISOString(),
        endTime = new Date(draft.end).toISOString();
      if (Date.parse(endTime) < Date.parse(startTime)) throw Error('结束时间不能早于开始时间');
      if (draft.kind === 'session' && draft.original) {
        const fields = {
          activityId: draft.activityId,
          sessionId: draft.sessionId,
          startTime,
          endTime,
          progress: draft.progress,
          segments: resizedSegments(draft.original, startTime, endTime),
        };
        if (draft.resumeToken)
          await dispatch('tracking', 'finish-session-edit', {
            ...fields,
            editToken: draft.resumeToken,
            cancel: false,
          });
        else await dispatch('tracking', 'edit-session', fields);
      } else {
        if (!draft.activityId && !draft.name.trim()) throw Error('先写下事项名称');
        await dispatch('tracking', 'add-session', {
          activityId: draft.activityId || null,
          name: draft.name,
          startTime,
          endTime,
          progress: draft.progress,
        });
      }
    }
    ui.editor = null;
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    saving.value = false;
  }
}
</script>
<template>
  <Modal
    v-if="ui.editor"
    :title="title"
    :class="{ 'session-editor': sessionForm }"
    @close="requestClose"
    ><form
      novalidate
      @submit.prevent="save"
      @keydown="todoKey"
      @compositionend="compositionEnd"
      @compositionstart="composing = true"
    >
      <div class="editor-body" :inert="saving || sessionEditorBusy">
        <select
          v-if="ui.editor.kind === 'record'"
          v-model="ui.editor.activityId"
          aria-label="补录到哪个事项"
        >
          <option :value="undefined">＋ 新建事项</option>
          <option v-for="matter in state?.tracking.activities" :key="matter.id" :value="matter.id">
            {{ matter.name }}
          </option>
        </select>
        <input
          v-if="
            ui.editor.kind === 'todo' ||
            ui.editor.kind === 'matter' ||
            (ui.editor.kind === 'record' && !ui.editor.activityId)
          "
          v-model="ui.editor.name"
          :maxlength="ui.editor.kind === 'todo' ? 80 : 200"
          aria-label="名称"
          placeholder="名称"
          autofocus
        />
        <template v-if="ui.editor.kind === 'todo'">
          <textarea
            v-model="ui.editor.notes"
            rows="3"
            maxlength="2000"
            aria-label="备注"
            placeholder="备注（可选）&#10;Enter 换行 · Ctrl+Enter 保存"
          />
          <select v-if="recurring" v-model="ui.editor.scope" aria-label="修改范围">
            <option value="once">仅本次</option>
            <option value="future">本次及以后</option>
          </select></template
        >
        <select v-if="ui.editor.kind === 'matter'" v-model="ui.editor.status" aria-label="事项状态">
          <option value="running">进行中</option>
          <option value="paused">已暂停</option>
          <option value="completed">已完成</option>
        </select>
        <template v-if="ui.editor.kind === 'session' || ui.editor.kind === 'record'"
          ><input
            v-model="ui.editor.progress"
            maxlength="2000"
            aria-label="本次进展"
            placeholder="本次进展（可选）"
          />
          <SessionTimeFields v-model:start="ui.editor.start" v-model:end="ui.editor.end" />
        </template>
        <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
      </div>
      <footer
        class="editor-buttons"
        :class="{
          'todo-editor-buttons': ui.editor.kind === 'todo',
          'session-editor-buttons': sessionForm,
        }"
      >
        <div v-if="ui.editor.kind === 'todo'" class="editor-option-row">
          <label><input v-model="ui.editor.urgent" type="checkbox" />紧急</label>
          <DatePicker v-if="ui.editor.scope === 'once'" v-model="ui.editor.deadline" />
        </div>
        <output v-if="sessionForm" class="editor-duration" aria-live="polite"
          >时长 {{ previewDuration || '—' }}</output
        >
        <button
          type="button"
          :disabled="saving || sessionEditorBusy || (sessionForm && !!busy)"
          @click="requestClose"
        >
          取消</button
        ><button class="primary" :disabled="saving || sessionEditorBusy || !!busy">保存</button>
      </footer>
    </form></Modal
  >
</template>
<style scoped>
.session-editor :deep(.editor-heading strong) {
  min-width: 0;
  overflow-wrap: anywhere;
}
.session-editor :deep(.editor-heading button) {
  flex-shrink: 0;
}
.session-editor .editor-body {
  padding: 12px 10px;
  min-width: 0;
}
.session-editor .editor-body > input,
.session-editor .editor-body > select {
  min-width: 0;
  font-size: inherit;
}
.session-editor-buttons {
  flex-wrap: wrap;
  gap: 6px;
  padding: 10px;
}
.session-editor-buttons .editor-duration {
  flex: 1 1 80px;
  min-width: 0;
  margin-right: auto;
  text-align: left;
  overflow-wrap: anywhere;
}
.session-editor-buttons button {
  flex-shrink: 0;
  padding: 7px 9px;
  font-size: inherit;
}
</style>
