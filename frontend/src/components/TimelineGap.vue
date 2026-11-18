<script setup lang="ts">
import Icon from './Icon.vue';
import { human, time } from '../display';
import type { GapRow } from '../timeline';
defineProps<{ row: GapRow; disabled?: boolean }>();
const emit = defineEmits<{ record: [] }>();
</script>
<template>
  <component
    :is="row.kind === 'gap' ? 'button' : 'div'"
    class="timeline-interval"
    :class="{ 'away-interval': row.kind === 'away' }"
    :data-interval-kind="row.kind"
    :disabled="row.kind === 'gap' ? disabled : undefined"
    :aria-label="row.kind === 'gap' ? `补录未计时 ${human(row.end - row.start)}` : '离开本事项'"
    @click="row.kind === 'gap' && emit('record')"
  >
    <span class="interval-times"
      ><time>{{ time(new Date(row.start).toISOString()) }}</time
      ><time>{{ time(new Date(row.end).toISOString()) }}</time></span
    >
    <i class="interval-bar" aria-hidden="true" />
    <span class="interval-body"
      ><span>{{ row.kind === 'gap' ? '未计时' : '离开本事项' }}</span
      ><small>{{ human(row.end - row.start) }}</small
      ><Icon v-if="row.kind === 'gap'" name="plus"
    /></span>
  </component>
</template>
<style scoped>
.timeline-interval {
  display: grid;
  grid-template-columns: 30px 3px minmax(0, 1fr);
  gap: 5px;
  align-items: stretch;
  width: 100%;
  min-width: 0;
  padding: 9px 0;
  border-bottom: 1px solid var(--border);
  color: var(--text-done);
  text-align: left;
}
.interval-times {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: flex-end;
  font-size: 9px;
  white-space: nowrap;
  padding: 2px 0;
}
.interval-bar {
  border-left: 3px dashed #c7d0bf;
  min-height: 36px;
}
.interval-body {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  min-width: 0;
  padding: 0 6px;
}
.interval-body > span {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.interval-body small {
  font-size: 10px;
  white-space: nowrap;
}
.away-interval {
  opacity: 0.75;
}
</style>
