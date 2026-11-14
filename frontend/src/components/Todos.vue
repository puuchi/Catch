<script setup lang="ts">
import { computed, ref } from 'vue';
import Icon from './Icon.vue';
import Resources from './Resources.vue';
import ValidationHint from './ValidationHint.vue';
import { busy, dispatch, editor, inform, now, state, ui } from '../store';
import { day, visibleTodos } from '../display';
import type { Todo } from '../models';
import { useScroll } from '../scroll';
const validation = ref('');
const scroll = ref<HTMLElement>();
const saveScroll = useScroll(scroll, () => (ui.history ? 'history' : 'todos'));
const rows = computed(() => {
  if (!state.value) return [];
  if (!ui.history) return visibleTodos(state.value.todos.todos, day(now.value));
  const byId = new Map(state.value.todos.archive.filter((t) => !t.deletedAt).map((t) => [t.id, t]));
  for (const row of state.value.todos.todos) if (row.done && !row.deletedAt) byId.set(row.id, row);
  return [...byId.values()].sort((a, b) =>
    (b.completedAt || '').localeCompare(a.completedAt || ''),
  );
});
const activeRepeat = (todo: Todo) =>
  state.value?.recurrences.templates.find(
    (t) => t.id === todo.recurrence?.templateId && !t.stopped,
  );
function repeat(todo?: Todo) {
  const template = todo && activeRepeat(todo);
  ui.repeatDraft = {
    mode: template ? 'template' : todo ? 'convert' : 'composer',
    todoId: todo?.id,
    templateId: template?.id,
    schedule: JSON.parse(
      JSON.stringify(template?.schedule || ui.composerSchedule || { kind: 'daily' }),
    ),
    startsOn: template?.startsOn || ui.composerStartsOn || day(now.value),
  };
}
async function create() {
  if (!ui.composer.text.trim()) {
    validation.value = '先记下一件想做的事';
    return;
  }
  try {
    const fields = { ...ui.composer, deadline: ui.composer.deadline || null };
    if (ui.composerSchedule)
      await dispatch('recurrence', 'create', {
        fields,
        resources: [],
        schedule: ui.composerSchedule,
        startsOn: ui.composerStartsOn || day(now.value),
      });
    else await dispatch('todos', 'create', fields);
    ui.composer = { text: '', notes: '', urgent: false, deadline: null };
    ui.composerExpanded = false;
    ui.composerSchedule = null;
  } catch (error) {
    validation.value = (error as Error).message;
  }
}
async function action(type: string, payload: Record<string, unknown>) {
  try {
    await dispatch('todos', type, payload);
  } catch (error) {
    inform(error);
  }
}
async function start(todo: Todo) {
  try {
    await dispatch('tracking', 'start', { name: todo.text, todoId: todo.id });
    ui.page = 'current';
  } catch (error) {
    inform(error);
  }
}
function edit(todo: Todo) {
  ui.editor = editor({
    kind: 'todo',
    id: todo.id,
    name: todo.text,
    notes: todo.notes,
    urgent: todo.urgent,
    deadline: todo.deadline,
  });
}
function toggle(id: string) {
  ui.expandedTodos = ui.expandedTodos.includes(id)
    ? ui.expandedTodos.filter((value) => value !== id)
    : [...ui.expandedTodos, id];
}
</script>
<template>
  <section class="module-page todos-page" aria-label="待办">
    <form
      v-if="!ui.history"
      id="todoComposer"
      :class="{ expanded: ui.composerExpanded }"
      novalidate
      @submit.prevent="create"
    >
      <div class="input-area field-wrap">
        <input
          v-model="ui.composer.text"
          id="todoInput"
          placeholder="记下一件事…"
          aria-label="待办内容"
          maxlength="80"
          autocomplete="off"
          @focus="ui.composerExpanded = true"
          @input="validation = ''"
        /><kbd aria-hidden="true">Enter</kbd
        ><ValidationHint :message="validation" @dismiss="validation = ''" />
      </div>
      <div v-if="ui.composerExpanded" class="composer-details">
        <textarea
          v-model="ui.composer.notes"
          id="todoNotes"
          rows="2"
          maxlength="2000"
          placeholder="备注（可选）"
          aria-label="备注"
        />
        <div class="composer-options">
          <button
            class="icon-button"
            type="button"
            title="标记紧急"
            aria-label="标记紧急"
            :aria-pressed="ui.composer.urgent"
            :class="{ active: ui.composer.urgent }"
            @click="ui.composer.urgent = !ui.composer.urgent"
          >
            <Icon name="flag" /></button
          ><label class="date-option icon-button" title="截止日期"
            ><Icon name="calendar" /><input
              v-model="ui.composer.deadline"
              type="date"
              aria-label="截止日期" /></label
          ><button
            type="button"
            class="repeat-option quiet-button"
            :class="{ active: ui.composerSchedule }"
            @click="repeat()"
          >
            <Icon name="repeat" />{{ ui.composerSchedule ? '已重复' : '重复' }}</button
          ><span class="spacer" /><button
            type="button"
            class="quiet-button"
            @click="ui.composerExpanded = false"
          >
            收起</button
          ><button class="primary" :disabled="!!busy">创建</button>
        </div>
      </div>
    </form>
    <ul id="todoList" ref="scroll" aria-label="待办列表" @scroll="saveScroll">
      <li
        v-for="todo in rows"
        :key="todo.id"
        class="todo-item"
        :class="{
          done: todo.done,
          urgent: todo.urgent,
          'deadline-today': todo.deadline === day(now),
        }"
        :data-resource-kind="'todo'"
        :data-resource-id="todo.id"
      >
        <input
          v-if="!ui.history"
          type="checkbox"
          :checked="todo.done"
          :aria-label="`${todo.done ? '撤销完成' : '完成'} ${todo.text}`"
          :disabled="!!busy"
          @change="action('complete', { id: todo.id, done: !todo.done })"
        />
        <div class="todo-content">
          <div class="todo-main">
            <button
              class="todo-text"
              :aria-expanded="ui.expandedTodos.includes(todo.id)"
              @click="toggle(todo.id)"
              @dblclick="!ui.history && edit(todo)"
            >
              {{ todo.text }}</button><span
              v-if="todo.deadline"
              class="deadline-badge"
              :class="{
                today: todo.deadline === day(now),
                overdue: todo.deadline < day(now) && !todo.done,
              }"
              >{{ todo.deadline.slice(5).replace('-', '月') }}日</span
            ><button
              v-if="todo.recurrence"
              class="repeat-mark quiet-button"
              :title="activeRepeat(todo) ? '重复规则' : '已停止重复'"
              :disabled="!activeRepeat(todo)"
              @click="repeat(todo)"
            >
              <Icon name="repeat" /></button
            ><span v-if="ui.history" class="muted small">{{ todo.completedAt }}</span
            ><button
              v-if="!ui.expandedTodos.includes(todo.id) && (todo.notes || todo.resources?.length)"
              class="quiet-button small"
              aria-label="展开备注和资料"
              @click="toggle(todo.id)"
            >
              {{ todo.notes ? '备注' : ''
              }}{{ todo.resources?.length ? ` · ${todo.resources.length} 份资料` : '' }}
            </button>
          </div>
          <template v-if="ui.expandedTodos.includes(todo.id)"
            ><p v-if="todo.notes" class="todo-notes">{{ todo.notes }}</p>
            <Resources :owner="{ kind: 'todo', id: todo.id }" :resources="todo.resources" /><button
              v-if="!ui.history && !todo.recurrence"
              class="quiet-button small"
              @click="repeat(todo)"
            >
              <Icon name="repeat" />设置重复
            </button></template
          >
        </div>
        <div v-if="!ui.history" class="row-actions">
          <button
            v-if="!todo.done"
            class="todo-start"
            :disabled="!!busy"
            :aria-label="`开始 ${todo.text}`"
            title="开始投入"
            @click="start(todo)"
          >
            <Icon name="play" /></button
          ><button
            class="icon-button row-edit"
            aria-label="编辑待办"
            title="编辑"
            @click="edit(todo)"
          >
            <Icon name="edit" /></button
          ><button
            class="todo-delete"
            :disabled="!!busy"
            :aria-label="`删除 ${todo.text}`"
            title="移入回收站"
            @click="action('trash', { id: todo.id })"
          >
            <Icon name="trash" />
          </button>
        </div>
      </li>
      <li v-if="!rows.length" class="empty-state">
        {{ ui.history ? '还没有完成的记录' : '留一点空白，也很好。' }}
      </li>
    </ul>
    <footer class="footer">
      <button @click="ui.history = !ui.history">{{ ui.history ? '返回待办' : '历史' }}</button><span
        class="muted small"
        >{{
          ui.history
            ? `${rows.length} 项记录`
            : rows.filter((t) => !t.done).length
              ? `${rows.filter((t) => !t.done).length} 件待办`
              : ''
        }}</span
      ><button
        v-if="!ui.history"
        class="icon-button"
        title="归档已完成"
        aria-label="归档已完成"
        :disabled="!!busy"
        @click="action('archive', {})"
      >
        <Icon name="broom" />
      </button>
    </footer>
  </section>
</template>
