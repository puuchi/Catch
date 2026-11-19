<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import DateField from './DateField.vue';
import Icon from './Icon.vue';
const from = defineModel<string>('from', { required: true });
const to = defineModel<string>('to', { required: true });
const kind = defineModel<'all' | 'todo' | 'matter' | 'session'>('kind', { default: 'all' });
defineProps<{ types?: boolean; count: number; dateLabel: string; target: string }>();
const expanded = ref(false);
const trigger = ref<HTMLButtonElement>();
const id = useId();
const active = computed(() => !!from.value || !!to.value);
const range = computed(() => `${from.value || '…'} — ${to.value || '…'}`);
function close() {
  expanded.value = false;
  trigger.value?.focus({ preventScroll: true });
}
const options = [
  { id: 'all', label: '全部', icon: 'list' },
  { id: 'todo', label: '待办', icon: 'check' },
  { id: 'matter', label: '事项', icon: 'folder' },
  { id: 'session', label: '单次记录', icon: 'timer' },
] as const;
</script>
<template>
  <Teleport defer :to="target">
    <button
      ref="trigger"
      class="icon-button history-date-toggle"
      :class="{ active }"
      type="button"
      v-tooltip="active ? `${dateLabel}筛选：${range}（${count}条）` : '日期筛选'"
      aria-label="日期筛选"
      :aria-expanded="expanded"
      :aria-controls="id"
      @click="expanded = !expanded"
    >
      <Icon name="calendar" />
    </button>
    <button
      v-if="active"
      class="icon-button"
      type="button"
      v-tooltip="'清除日期筛选'"
      aria-label="清除日期筛选"
      @click="
        from = '';
        to = '';
      "
    >
      <Icon name="close" />
    </button>
  </Teleport>
  <div v-if="types || expanded" class="history-filters">
    <div v-if="types" class="history-types" role="group" aria-label="回收站类型">
      <button
        v-for="option in options"
        :key="option.id"
        type="button"
        :aria-pressed="kind === option.id"
        @click="kind = option.id"
      >
        <Icon :name="option.icon" />{{ option.label }}
      </button>
    </div>
    <div
      v-if="expanded"
      :id="id"
      class="history-date-panel"
      role="group"
      aria-label="日期范围筛选"
      @keydown.esc.stop.prevent="close"
    >
      <div class="history-date-range">
        <DateField v-model="from" :label="`${dateLabel}起始`" placeholder="起始日期" />
        <span aria-hidden="true">—</span>
        <DateField v-model="to" :label="`${dateLabel}截止`" placeholder="截止日期" />
      </div>
      <p v-if="from && to && from > to" class="editor-error" role="alert">
        起始日期不能晚于截止日期
      </p>
      <div class="history-date-actions">
        <button
          type="button"
          :disabled="!active"
          @click="
            from = '';
            to = '';
          "
        >
          清除</button
        ><button type="button" @click="close">收起</button>
      </div>
    </div>
  </div>
</template>
<style scoped>
.history-filters {
  flex: 0 0 auto;
  padding: 5px 12px 6px;
}
.history-types {
  display: grid;
  grid-template-columns: 0.85fr 1fr 1fr 1.45fr;
  gap: 3px;
}
.history-types button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  padding: 5px 2px;
  border: 1px solid var(--border);
  border-radius: 7px;
  font-size: 9px;
  color: var(--text-muted);
  white-space: nowrap;
}
.history-types button[aria-pressed='true'] {
  background: #dce7d7;
  border-color: #9cb59b;
  color: var(--accent);
  font-weight: 600;
}
.history-types .ui-icon {
  width: 12px;
  height: 12px;
}
.history-date-toggle.active {
  background: #dce7d7;
  color: var(--accent);
  box-shadow: inset 0 0 0 1px #a7b99c;
}
.history-date-panel {
  margin-top: 5px;
  padding: 8px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: #edf1e680;
}
.history-date-range {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 8px minmax(0, 1fr);
  align-items: center;
  gap: 3px;
}
.history-date-range > span {
  font-size: 9px;
  color: var(--text-muted);
}
.history-date-range :deep(.date-control input) {
  font-size: 10px;
  padding: 6px 2px;
}
.history-date-range :deep(.date-control button) {
  flex-basis: 18px;
  width: 18px;
}
.history-date-range :deep(.ui-icon) {
  width: 12px;
  height: 12px;
}
.history-date-panel .editor-error {
  margin-top: 5px;
}
.history-date-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 6px;
}
.history-date-actions button {
  font-size: 10px;
  color: var(--accent);
  padding: 2px;
}
</style>
