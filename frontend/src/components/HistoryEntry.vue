<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import Icon from './Icon.vue';
import { historyDate } from '../history';
const props = defineProps<{
  kind: 'todo' | 'matter' | 'session';
  title: string;
  date?: string | null;
  dateLabel: string;
  note?: string;
  detail?: string;
  locked?: boolean;
  removable?: boolean;
  hasDetails?: boolean;
}>();
defineEmits<{ restore: []; remove: [] }>();
const expanded = ref(false);
const id = useId();
const expandable = computed(() => !!props.note?.trim() || !!props.hasDetails);
const types = {
  todo: { icon: 'check', label: '待办' },
  matter: { icon: 'folder', label: '事项' },
  session: { icon: 'timer', label: '单次记录' },
};
</script>
<template>
  <article
    class="history-entry"
    :class="[`history-entry-${kind}`, { expanded: expanded && expandable }]"
    :data-history-kind="kind"
  >
    <span
      class="history-type-mark"
      :title="types[kind].label"
      :aria-label="types[kind].label"
      role="img"
      ><Icon :name="types[kind].icon"
    /></span>
    <h3>
      <button
        v-if="expandable"
        class="history-entry-title"
        type="button"
        :title="title"
        :aria-expanded="expanded"
        :aria-controls="id"
        @click="expanded = !expanded"
      >
        <span>{{ title }}</span>
      </button>
      <span v-else class="history-entry-title history-entry-static" :title="title"
        ><span>{{ title }}</span></span
      >
    </h3>
    <div class="history-entry-actions">
      <button
        class="icon-button"
        :title="kind === 'todo' && !removable ? '恢复到待办' : '恢复'"
        :aria-label="`恢复：${title}`"
        :disabled="locked"
        @click="$emit('restore')"
      >
        <Icon name="restore" />
      </button>
      <button
        v-if="removable"
        class="icon-button history-remove"
        title="永久删除"
        :aria-label="`永久删除：${title}`"
        :disabled="locked"
        @click="$emit('remove')"
      >
        <Icon name="trash" />
      </button>
    </div>
    <div class="history-entry-meta">
      <time :datetime="date || undefined" :title="dateLabel">{{
        historyDate(date).replaceAll('-', '/') || '日期未记录'
      }}</time><span v-if="detail">{{ detail }}</span>
    </div>
    <div v-if="expanded && expandable" :id="id" class="history-entry-details">
      <p v-if="note" class="history-entry-note">{{ note }}</p>
      <slot />
    </div>
  </article>
</template>
<style scoped>
.history-entry {
  display: grid;
  grid-template-columns: 20px minmax(0, 1fr) auto;
  gap: 2px 6px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  align-items: start;
  --kind-color: #587451;
  --kind-wash: #e8eedf;
}
.history-entry-matter {
  --kind-color: #527a85;
  --kind-wash: #e3ecec;
}
.history-entry-session {
  --kind-color: #9b7b45;
  --kind-wash: #f1eadb;
}
.history-type-mark {
  grid-column: 1;
  grid-row: 1 / span 2;
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 6px;
  color: var(--kind-color);
  background: var(--kind-wash);
  margin-top: 1px;
}
.history-entry-matter .history-type-mark {
  border-radius: 4px;
}
.history-entry-session .history-type-mark {
  border-radius: 50%;
}
.history-type-mark .ui-icon {
  width: 12px;
  height: 12px;
}
.history-entry h3 {
  min-width: 0;
  font-size: 11px;
  line-height: 1.5;
  font-weight: 600;
}
.history-entry-title {
  display: flex;
  align-items: center;
  gap: 3px;
  text-align: left;
  width: 100%;
  min-width: 0;
  min-height: 22px;
  padding: 0;
  color: inherit;
  font: inherit;
}
.history-entry-title span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.history-entry-session h3 {
  font-weight: 500;
}
.history-entry-meta {
  grid-column: 2 / -1;
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 2px 8px;
  font-size: 9px;
  line-height: 1.5;
  color: var(--text-muted);
}
.history-entry-meta > span {
  color: var(--kind-color);
}
.history-entry-details {
  grid-column: 2 / -1;
  min-width: 0;
}
.history-entry-note {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-size: 10px;
  line-height: 1.6;
  color: var(--text-muted);
  margin-top: 5px;
}
.history-entry-actions {
  display: flex;
  gap: 2px;
}
.history-entry-actions .icon-button {
  width: 22px;
  height: 22px;
  padding: 3px;
}
.history-entry-actions .ui-icon {
  width: 14px;
  height: 14px;
}
.history-entry-actions button:first-child {
  color: var(--accent);
}
.history-remove:hover {
  color: #af563f;
  background: #f4e8e1;
}
</style>
