<script setup lang="ts">
import { computed, ref } from 'vue';
import Icon from './Icon.vue';
import Modal from './Modal.vue';
import HistoryFilters from './HistoryFilters.vue';
import HistoryEntry from './HistoryEntry.vue';
import { busy, dispatch, inform, now, state, ui } from '../store';
import { human, duration } from '../display';
import { inDateRange } from '../history';
import { useScroll } from '../scroll';
const scroll = ref<HTMLElement>();
const from = ref(''),
  to = ref('');
const saving = ref(false);
const locked = computed(() => saving.value || !!busy.value);
const saveScroll = useScroll(scroll, () => `trash:${ui.trashFilter}:${from.value}:${to.value}`);
const pending = ref<{ group: 'todos' | 'tracking'; id: string; name: string } | null>(null);
const rows = computed(() =>
  [
    ...(state.value?.todos.todos.filter((t) => t.deletedAt) || []).map((todo) => ({
      id: todo.id,
      group: 'todos' as const,
      kind: 'todo' as const,
      title: todo.text,
      deletedAt: todo.deletedAt!,
      note: todo.notes,
      detail: '',
    })),
    ...(state.value?.tracking.trash || []).map((entry) => ({
      id: entry.id,
      group: 'tracking' as const,
      kind: entry.kind,
      title: entry.activity.name,
      deletedAt: entry.deletedAt,
      note: entry.session?.progress || '',
      detail:
        entry.kind === 'matter'
          ? `${entry.activity.sessions.length} 次投入 · ${human(duration(entry.activity, now.value))}`
          : human(duration(entry.session || entry.activity, now.value)),
    })),
  ]
    .filter(
      (row) =>
        (ui.trashFilter === 'all' || ui.trashFilter === row.kind) &&
        inDateRange(row.deletedAt, from.value, to.value),
    )
    .sort((a, b) => b.deletedAt.localeCompare(a.deletedAt)),
);
async function action(group: 'todos' | 'tracking', type: string, id: string) {
  if (locked.value) return;
  saving.value = true;
  try {
    await dispatch(group, type, { id });
    pending.value = null;
  } catch (error) {
    inform(error, type === 'restore' || type === 'restore-trash' ? 2000 : 0);
  } finally {
    saving.value = false;
  }
}
</script>
<template>
  <section class="module-page recycle-page" aria-label="回收站">
    <header class="archive-heading">
      <button class="icon-button" aria-label="返回" title="返回" @click="ui.page = ui.previousPage">
        <Icon name="back" /></button
      ><strong>回收站</strong>
      <span id="trash-filter-tools" class="history-heading-tools" />
    </header>
    <HistoryFilters
      v-model:from="from"
      v-model:to="to"
      v-model:kind="ui.trashFilter"
      types
      :count="rows.length"
      target="#trash-filter-tools"
      date-label="删除日期"
    />
    <div ref="scroll" class="recycle-list" @scroll="saveScroll">
      <HistoryEntry
        v-for="row in rows"
        :key="`${row.group}:${row.id}`"
        :kind="row.kind"
        :title="row.title"
        :date="row.deletedAt"
        date-label="删除日期"
        :note="row.note"
        :detail="row.detail"
        :locked="locked"
        removable
        @restore="action(row.group, row.group === 'todos' ? 'restore' : 'restore-trash', row.id)"
        @remove="pending = { group: row.group, id: row.id, name: row.title }"
      />
      <p v-if="!rows.length" class="empty-state">
        {{ from || to || ui.trashFilter !== 'all' ? '没有符合筛选的记录。' : '回收站是空的。' }}
      </p>
    </div>
    <Modal v-if="pending" title="永久删除？" @close="!locked && (pending = null)">
      <div class="editor-body">
        <p class="confirm-name">{{ pending.name }}</p>
        <p class="muted small">删除后无法恢复。</p>
      </div>
      <footer class="editor-buttons">
        <button :disabled="locked" @click="pending = null">取消</button>
        <button
          class="danger-button"
          :disabled="locked"
          @click="
            action(pending.group, pending.group === 'todos' ? 'delete' : 'delete-trash', pending.id)
          "
        >
          永久删除
        </button>
      </footer>
    </Modal>
  </section>
</template>
