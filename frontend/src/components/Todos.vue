<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import Icon from './Icon.vue';
import Resources from './Resources.vue';
import ValidationHint from './ValidationHint.vue';
import ActionMenu from './ActionMenu.vue';
import DatePicker from './DatePicker.vue';
import HistoryFilters from './HistoryFilters.vue';
import HistoryEntry from './HistoryEntry.vue';
import { inDateRange } from '../history';
import { host } from '../bridge';
import {
  attachComposerResources,
  busy,
  dispatch,
  editor,
  generation,
  inform,
  now,
  state,
  ui,
} from '../store';
import { day, visibleTodos } from '../display';
import type { ResourceKind, Todo } from '../models';
import { useScroll } from '../scroll';
const validation = ref('');
const addingResources = ref(false);
async function addResources(kind: ResourceKind) {
  if (kind === 'web') {
    ui.resourceDraft = { owner: { kind: 'composer' }, mode: 'attach', kind, title: '', target: '' };
    return;
  }
  try {
    const composer = ui.composer;
    const paths = await host.pick(generation.value, kind);
    if (ui.composer !== composer) return;
    attachComposerResources(
      paths.map((target) => ({
        id: crypto.randomUUID(),
        kind,
        target,
        title:
          target
            .replace(/[\\/]+$/, '')
            .split(/[\\/]/)
            .at(-1) || target,
      })),
    );
  } catch (error) {
    validation.value = (error as Error).message;
  }
}
const menu = ref<{ todo: Todo; x: number; y: number; trigger?: HTMLElement } | null>(null);
let clickTimer: ReturnType<typeof setTimeout> | undefined;
let compositionEnded = -Infinity;
let composing = false;
function compositionEnd() {
  composing = false;
  compositionEnded = performance.now();
}
const scroll = ref<HTMLElement>();
const archiveFrom = ref(''),
  archiveTo = ref('');
const saveScroll = useScroll(scroll, () => (ui.history ? 'history' : 'todos'));
const rows = computed(() => {
  if (!state.value) return [];
  if (!ui.history) return visibleTodos(state.value.todos.todos, day(now.value));
  const byId = new Map(state.value.todos.archive.filter((t) => !t.deletedAt).map((t) => [t.id, t]));
  for (const row of state.value.todos.todos) if (row.done && !row.deletedAt) byId.set(row.id, row);
  return [...byId.values()]
    .filter((todo) => inDateRange(todo.completedAt, archiveFrom.value, archiveTo.value))
    .sort((a, b) => (b.completedAt || '').localeCompare(a.completedAt || ''));
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
  if (busy.value || composing || performance.now() - compositionEnded < 80) return;
  if (!ui.composer.text.trim()) {
    validation.value = '先记下一件想做的事';
    return;
  }
  try {
    const fields = { ...ui.composer, deadline: ui.composer.deadline || null };
    if (ui.composerSchedule)
      await dispatch('recurrence', 'create', {
        fields,
        resources: ui.composerResources || [],
        schedule: ui.composerSchedule,
        startsOn: ui.composerStartsOn || day(now.value),
      });
    else if (ui.composerResources?.length)
      await dispatch('todos', 'create-with-resources', { fields, resources: ui.composerResources });
    else await dispatch('todos', 'create', fields);
    ui.composer = { text: '', notes: '', urgent: false, deadline: null };
    ui.composerExpanded = false;
    ui.composerSchedule = null;
    ui.composerResources = [];
    addingResources.value = false;
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
  if (busy.value) return;
  try {
    const linked = (state.value?.tracking.activities || [])
      .filter((matter) => matter.todoId === todo.id && matter.status !== 'completed')
      .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt));
    const existing = linked.find((matter) => matter.status === 'running') || linked[0];
    if (existing?.status === 'paused') await dispatch('tracking', 'resume', { id: existing.id });
    else if (!existing) await dispatch('tracking', 'start', { name: todo.text, todoId: todo.id });
    if (existing) ui.foregroundId = existing.id;
    ui.page = 'current';
  } catch (error) {
    inform(error);
  }
}
function edit(todo: Todo) {
  clearTimeout(clickTimer);
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
  const collapsed = ui.collapsedTodos || [];
  ui.collapsedTodos = collapsed.includes(id)
    ? collapsed.filter((value) => value !== id)
    : [...collapsed, id];
}
function titleClick(event: MouseEvent, todo: Todo) {
  clearTimeout(clickTimer);
  if (event.detail === 0) toggle(todo.id);
  else if (event.detail === 1) clickTimer = setTimeout(() => toggle(todo.id), 250);
}
function showMenu(event: MouseEvent, todo: Todo) {
  if (ui.history || (event.target as HTMLElement).closest('input,textarea')) return;
  event.preventDefault();
  const row = (event.currentTarget as HTMLElement).closest('.todo-item');
  const trigger = row?.querySelector<HTMLElement>('.todo-menu-button') || undefined;
  const rect = trigger?.getBoundingClientRect();
  menu.value = {
    todo,
    x: event.type === 'contextmenu' ? event.clientX : rect?.left || 0,
    y: event.type === 'contextmenu' ? event.clientY : rect?.bottom || 0,
    trigger,
  };
}
function menuKey(event: KeyboardEvent, todo: Todo) {
  if (event.key !== 'ContextMenu' && !(event.shiftKey && event.key === 'F10')) return;
  event.preventDefault();
  const trigger = (event.currentTarget as HTMLElement).querySelector<HTMLButtonElement>(
    '.todo-menu-button',
  );
  trigger?.click();
}
function menuAction(id: string) {
  const todo = menu.value?.todo;
  if (!todo) return;
  if (id === 'edit') edit(todo);
  if (id === 'repeat') repeat(todo);
  if (id === 'trash') void action('trash', { id: todo.id });
}
function groupLabel(todo: Todo, index: number) {
  const label = ui.history ? (todo.completedAt || '').slice(0, 7) : todo.done ? '今日已完成' : '';
  const previous = rows.value[index - 1];
  const previousLabel = previous
    ? ui.history
      ? (previous.completedAt || '').slice(0, 7)
      : previous.done
        ? '今日已完成'
        : ''
    : '';
  return label !== previousLabel ? label : '';
}
function longNotes(todo: Todo) {
  return todo.notes.length > 55 || todo.notes.split('\n').length > 2;
}
async function restoreArchived(todo: Todo) {
  if (busy.value) return;
  try {
    await dispatch('todos', 'restore-archive', { id: todo.id });
    inform('已恢复到待办', 2000);
  } catch (error) {
    inform(error, 2000);
  }
}
function toggleNotes(id: string) {
  const expanded = ui.expandedTodoNotes || [];
  ui.expandedTodoNotes = expanded.includes(id)
    ? expanded.filter((value) => value !== id)
    : [...expanded, id];
}
function cancelComposer() {
  composing = false;
  compositionEnded = -Infinity;
  ui.composer = { text: '', notes: '', urgent: false, deadline: null };
  ui.composerExpanded = false;
  ui.composerSchedule = null;
  ui.composerResources = [];
  addingResources.value = false;
  validation.value = '';
}
function composerKey(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229 || performance.now() - compositionEnded < 80)
    return;
  if (event.key === 'Escape') {
    event.preventDefault();
    cancelComposer();
  }
  if (event.key !== 'Enter' || event.repeat) return;
  const target = event.target as HTMLElement;
  if (
    target.id === 'todoInput' ||
    (target.tagName === 'TEXTAREA' && (event.ctrlKey || event.metaKey))
  ) {
    event.preventDefault();
    void create();
  }
}
onBeforeUnmount(() => clearTimeout(clickTimer));
</script>
<template>
  <section class="module-page todos-page" aria-label="待办">
    <header v-if="ui.history" class="archive-heading">
      <button
        type="button"
        class="icon-button"
        title="返回待办"
        aria-label="返回待办"
        @click="ui.history = false"
      >
        <Icon name="back" />
      </button>
      <strong>归档</strong>
      <span id="archive-filter-tools" class="history-heading-tools" />
    </header>
    <HistoryFilters
      v-if="ui.history"
      v-model:from="archiveFrom"
      v-model:to="archiveTo"
      :count="rows.length"
      target="#archive-filter-tools"
      date-label="完成日期"
    />
    <form
      v-if="!ui.history"
      id="todoComposer"
      :class="{ expanded: ui.composerExpanded }"
      novalidate
      @submit.prevent="create"
      @keydown="composerKey"
      @compositionend="compositionEnd"
      @compositionstart="composing = true"
    >
      <div class="input-area field-wrap">
        <input
          v-model="ui.composer.text"
          id="todoInput"
          type="text"
          placeholder="添加待办…（Enter）"
          aria-label="待办内容"
          maxlength="80"
          autocomplete="off"
          @focus="ui.composerExpanded = true"
          @input="validation = ''"
        /><ValidationHint :message="validation" @dismiss="validation = ''" />
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
            title="紧急"
            aria-label="紧急"
            :aria-pressed="ui.composer.urgent"
            :class="{ active: ui.composer.urgent }"
            @click="ui.composer.urgent = !ui.composer.urgent"
          >
            <Icon name="flag" /></button
          ><DatePicker v-model="ui.composer.deadline" /><button
            type="button"
            class="repeat-option icon-button"
            :title="ui.composerSchedule ? '修改重复' : '重复'"
            aria-label="重复"
            :aria-pressed="!!ui.composerSchedule"
            :class="{ active: ui.composerSchedule }"
            @click="repeat()"
          >
            <Icon name="repeat" /></button
          ><button
            type="button"
            class="icon-button"
            title="添加资料"
            aria-label="添加资料"
            :aria-expanded="addingResources"
            :class="{ active: ui.composerResources?.length }"
            :disabled="!!busy"
            @click="addingResources = !addingResources"
          >
            <Icon name="clip" /><span
              v-if="ui.composerResources?.length"
              class="composer-resource-count"
              >{{ ui.composerResources.length }}</span
            ></button
          ><span class="spacer" /><button
            type="button"
            class="quiet-button"
            @click="cancelComposer"
          >
            取消</button
          ><button class="primary" :disabled="!!busy">创建</button>
        </div>
        <div v-if="addingResources" class="resource-options">
          <button type="button" :disabled="!!busy" @click="addResources('file')">文件</button
          ><button type="button" :disabled="!!busy" @click="addResources('folder')">文件夹</button
          ><button type="button" :disabled="!!busy" @click="addResources('web')">网页</button>
        </div>
        <div v-if="ui.composerResources?.length" class="composer-resource-list">
          <div v-for="resource in ui.composerResources" :key="resource.id" class="resource-row">
            <span class="composer-resource-name" :title="resource.target"
              ><Icon :name="resource.kind === 'web' ? 'link' : resource.kind" />{{
                resource.title
              }}</span
            ><button
              type="button"
              class="icon-button"
              title="移除"
              :aria-label="`移除${resource.title}`"
              :disabled="!!busy"
              @click="
                ui.composerResources = ui.composerResources.filter((row) => row.id !== resource.id)
              "
            >
              <Icon name="unlink" />
            </button>
          </div>
        </div>
      </div>
    </form>
    <ul
      id="todoList"
      ref="scroll"
      :aria-label="ui.history ? '归档列表' : '待办列表'"
      @scroll="saveScroll"
    >
      <template v-if="ui.history">
        <template v-for="(todo, index) in rows" :key="todo.id">
          <li v-if="groupLabel(todo, index)" class="section-divider">
            {{ groupLabel(todo, index).replace('-', ' 年 ') + ' 月' }}
          </li>
          <li>
            <HistoryEntry
              kind="todo"
              :title="todo.text"
              :date="todo.completedAt"
              date-label="完成日期"
              :note="todo.notes"
              :has-details="!!todo.resources?.length"
              :locked="!!busy"
              @restore="restoreArchived(todo)"
            >
              <Resources
                v-if="todo.resources?.length"
                :owner="{ kind: 'todo', id: todo.id }"
                :resources="todo.resources"
                readonly
                compact
              />
            </HistoryEntry>
          </li>
        </template>
      </template>
      <template v-else>
        <template v-for="(todo, index) in rows" :key="todo.id">
          <li v-if="groupLabel(todo, index)" class="section-divider">
            {{ ui.history ? groupLabel(todo, index).replace('-', ' 年 ') + ' 月' : '今日已完成' }}
            <button
              v-if="!ui.history"
              class="icon-button archive-completed"
              title="归档已完成"
              aria-label="归档已完成"
              :disabled="!!busy"
              @click="action('archive', {})"
            >
              <Icon name="broom" />
            </button>
          </li>
          <li
            class="todo-item"
            :class="{
              done: todo.done,
              urgent: todo.urgent,
              'deadline-today': todo.deadline === day(now),
            }"
            :data-resource-kind="'todo'"
            :data-resource-id="todo.id"
            @contextmenu="showMenu($event, todo)"
            @keydown="menuKey($event, todo)"
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
                  :aria-expanded="!ui.collapsedTodos?.includes(todo.id)"
                  @click="titleClick($event, todo)"
                  @dblclick="!ui.history && edit(todo)"
                >
                  {{ todo.text }}
                </button>
              </div>
              <div v-show="!ui.collapsedTodos?.includes(todo.id)" class="todo-details">
                <p
                  v-if="todo.notes"
                  class="todo-notes"
                  :class="{
                    'todo-notes-clamped':
                      longNotes(todo) && !ui.expandedTodoNotes?.includes(todo.id),
                  }"
                >
                  {{ todo.notes }}
                </p>
                <button
                  v-if="longNotes(todo)"
                  class="quiet-button small"
                  :aria-expanded="!!ui.expandedTodoNotes?.includes(todo.id)"
                  @click="toggleNotes(todo.id)"
                >
                  {{ ui.expandedTodoNotes?.includes(todo.id) ? '收起' : '展开' }}
                </button>
                <div class="todo-metadata">
                  <span
                    v-if="todo.deadline && !todo.done"
                    class="deadline-badge todo-deadline"
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
                    <Icon name="repeat" />
                  </button>
                  <Resources
                    :owner="{ kind: 'todo', id: todo.id }"
                    :resources="todo.resources"
                    :readonly="ui.history"
                    compact
                  />
                </div>
              </div>
            </div>
            <div v-if="ui.history" class="archived-actions">
              <time
                v-show="!ui.collapsedTodos?.includes(todo.id)"
                class="archive-date"
                :datetime="todo.completedAt || undefined"
                >{{ todo.completedAt }}</time
              >
              <button
                class="icon-button"
                title="恢复到待办"
                :aria-label="`恢复到待办：${todo.text}`"
                :disabled="!!busy"
                @click="restoreArchived(todo)"
              >
                <Icon name="restore" />
              </button>
            </div>
            <div v-else class="row-actions">
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
                class="icon-button todo-menu-button"
                :aria-label="`${todo.text}的操作`"
                title="待办操作"
                aria-haspopup="menu"
                :aria-expanded="menu?.todo.id === todo.id"
                @click="showMenu($event, todo)"
              >
                <Icon name="more" />
              </button>
            </div>
          </li>
        </template>
      </template>
      <li v-if="!rows.length" class="empty-state">
        {{
          ui.history
            ? archiveFrom || archiveTo
              ? '没有符合筛选的归档记录'
              : '还没有完成的记录'
            : '留一点空白，也很好。'
        }}
      </li>
    </ul>
  </section>
  <ActionMenu
    v-if="menu"
    :x="menu.x"
    :y="menu.y"
    :trigger="menu.trigger"
    label="待办操作"
    :items="[
      { id: 'edit', label: '编辑', icon: 'edit', disabled: !!busy },
      {
        id: 'repeat',
        label: activeRepeat(menu.todo) ? '修改重复' : '设置重复',
        icon: 'repeat',
        disabled: !!busy,
      },
      { id: 'trash', label: '移入回收站', icon: 'trash', danger: true, disabled: !!busy },
    ]"
    @select="menuAction"
    @close="menu = null"
  />
</template>
