<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import Modal from './Modal.vue';
import { busy, dispatch, state, ui } from '../store';
import { human, resizedSegments } from '../display';
const error = ref('');
const title = computed(() =>
  ui.editor?.kind === 'todo'
    ? '编辑待办'
    : ui.editor?.kind === 'matter'
      ? '编辑事项'
      : ui.editor?.kind === 'session'
        ? '编辑本次投入'
        : '补录投入',
);
watch(
  () => ui.editor,
  () => {
    error.value = '';
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
  const draft = ui.editor;
  if (!draft) return;
  error.value = '';
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
      if (draft.kind === 'session' && draft.original)
        await dispatch('tracking', 'edit-session', {
          activityId: draft.activityId,
          sessionId: draft.sessionId,
          startTime,
          endTime,
          progress: draft.progress,
          segments: resizedSegments(draft.original, startTime, endTime),
        });
      else {
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
  }
}
</script>
<template>
  <Modal v-if="ui.editor" :title="title" @close="ui.editor = null"
    ><form novalidate @submit.prevent="save">
      <div class="editor-body">
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
            placeholder="备注（可选）"
          />
          <div class="editor-option-row">
            <label><input v-model="ui.editor.urgent" type="checkbox" />紧急</label
            ><input
              v-if="ui.editor.scope === 'once'"
              v-model="ui.editor.deadline"
              type="date"
              aria-label="截止日期"
            />
          </div>
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
          /><label class="stack-field"
            >开始<input
              v-model="ui.editor.start"
              type="datetime-local"
              step="0.001"
              aria-label="开始时间" /></label
          ><label class="stack-field"
            >结束<input
              v-model="ui.editor.end"
              type="datetime-local"
              step="0.001"
              aria-label="结束时间" /></label
          ><output class="editor-duration">{{ previewDuration }}</output></template
        >
        <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
      </div>
      <footer class="editor-buttons">
        <button type="button" @click="ui.editor = null">取消</button
        ><button class="primary" :disabled="!!busy">保存</button>
      </footer>
    </form></Modal
  >
</template>
