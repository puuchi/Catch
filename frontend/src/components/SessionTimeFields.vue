<script setup lang="ts">
import { computed } from 'vue';
import DateField from './DateField.vue';
import TimeField from './TimeField.vue';
import { calendarOffset, offsetDate, withDate } from './session-time';
const props = defineProps<{ start: string; end: string }>();
const emit = defineEmits<{ 'update:start': [value: string]; 'update:end': [value: string] }>();
const offset = computed(() => calendarOffset(props.start, props.end));
const date = computed({
  get: () => props.start.slice(0, 10),
  set: (value: string) => {
    const endDate = value ? offsetDate(value, offset.value) : '';
    emit('update:start', withDate(props.start, value));
    emit('update:end', withDate(props.end, endDate));
  },
});
function changeDay(event: Event) {
  const day = Number((event.target as HTMLSelectElement).value);
  emit('update:end', withDate(props.end, offsetDate(props.start.slice(0, 10), day)));
}
</script>
<template>
  <div class="session-time-fields">
    <div class="session-date-row">
      <DateField v-model="date" />
      <select
        class="end-day"
        :value="offset"
        aria-label="结束日期"
        :disabled="!start"
        @change="changeDay"
      >
        <option :value="0">当天</option>
        <option :value="1">次日</option>
        <option v-if="offset > 1" :value="offset">第 {{ offset + 1 }} 天</option>
        <option v-if="offset < 0" :value="offset">早于开始</option>
      </select>
    </div>
    <div class="time-columns">
      <TimeField
        :model-value="start"
        label="开始"
        hide-label
        @update:model-value="emit('update:start', $event)"
      />
      <TimeField
        :model-value="end"
        label="结束"
        hide-label
        @update:model-value="emit('update:end', $event)"
      />
    </div>
  </div>
</template>
<style scoped>
.session-time-fields {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}
.session-date-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
}
.time-columns {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 8px;
}
.end-day {
  width: auto;
  max-width: 90px;
  min-width: 0;
  padding: 6px 4px;
  font-size: 10px;
  color: var(--text-muted);
}
</style>
