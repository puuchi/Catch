<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId } from 'vue';
import { now } from '../store';
import { inputTime } from '../display';
import { withTime } from './session-time';
import Icon from './Icon.vue';
const value = defineModel<string>({ required: true });
defineProps<{ label: string; hideLabel?: boolean }>();
const id = useId();
const picker = ref<HTMLDialogElement>();
const milliseconds = ref('');
const fields = [
  { key: 'year', name: '年', min: 1, max: 9999 },
  { key: 'month', name: '月', min: 1, max: 12 },
  { key: 'day', name: '日', min: 1, max: 31 },
  { key: 'hour', name: '时', min: 0, max: 23 },
  { key: 'minute', name: '分', min: 0, max: 59 },
  { key: 'second', name: '秒', min: 0, max: 59 },
] as const;
type FieldKey = (typeof fields)[number]['key'];
const parts = ref<Record<FieldKey, string | number>>({
  year: '',
  month: '',
  day: '',
  hour: '',
  minute: '',
  second: '',
});
const draft = computed(() => {
  const p = (key: FieldKey, width = 2) => String(parts.value[key]).padStart(width, '0');
  return `${p('year', 4)}-${p('month')}-${p('day')}T${p('hour')}:${p('minute')}:${p('second')}${milliseconds.value}`;
});
const valid = computed(() => {
  if (
    !fields.every((field) => {
      const n = Number(parts.value[field.key]);
      return (
        parts.value[field.key] !== '' && Number.isInteger(n) && n >= field.min && n <= field.max
      );
    })
  )
    return false;
  const parsed = new Date(draft.value);
  return (
    Number.isFinite(+parsed) &&
    parsed.getFullYear() === +parts.value.year &&
    parsed.getMonth() + 1 === +parts.value.month &&
    parsed.getDate() === +parts.value.day &&
    parsed.getHours() === +parts.value.hour &&
    parsed.getMinutes() === +parts.value.minute &&
    parsed.getSeconds() === +parts.value.second
  );
});
function open() {
  const source = value.value || inputTime(new Date(now.value).toISOString());
  parts.value = {
    year: source.slice(0, 4),
    month: source.slice(5, 7),
    day: source.slice(8, 10),
    hour: source.slice(11, 13),
    minute: source.slice(14, 16),
    second: source.slice(17, 19),
  };
  milliseconds.value = source.slice(19);
  picker.value?.showModal();
}
function close() {
  picker.value?.close();
}
function confirm() {
  if (!valid.value) return;
  value.value = draft.value;
  close();
}
function updateTime(event: Event) {
  value.value = withTime(
    value.value || inputTime(new Date(now.value).toISOString()),
    (event.target as HTMLInputElement).value,
  );
}
function pickerKey(event: KeyboardEvent) {
  event.stopPropagation();
  if (event.key === 'Enter' && (event.target as HTMLElement).tagName === 'INPUT') {
    event.preventDefault();
    confirm();
  }
}
onBeforeUnmount(close);
</script>
<template>
  <div class="time-field">
    <label v-if="!hideLabel" :for="id">{{ label }}</label>
    <div class="time-control">
      <input
        :id="id"
        type="time"
        step="1"
        :value="value.slice(11, 19)"
        :aria-label="`${label}时间`"
        @input="updateTime"
      />
      <div class="time-actions">
        <button
          type="button"
          :aria-label="`选择${label}日期和时间`"
          v-tooltip="`选择${label}日期和时间`"
          @click="open"
        >
          <Icon name="timer" />
        </button>
        <button
          type="button"
          v-tooltip="`${label}设为现在`"
          :aria-label="`${label}设为现在`"
          @click="value = inputTime(new Date(now).toISOString())"
        >
          现在
        </button>
      </div>
    </div>
    <dialog
      ref="picker"
      class="time-popover"
      :aria-label="`${label}日期和时间`"
      @cancel.prevent.stop="close"
      @click.stop="$event.target === picker && close()"
      @keydown="pickerKey"
    >
      <header>{{ label }}时间</header>
      <div class="time-parts">
        <label v-for="field in fields" :key="field.key"
          >{{ field.name
          }}<input
            v-model="parts[field.key]"
            type="number"
            :min="field.min"
            :max="field.max"
            step="1"
            :aria-label="`${label}${field.name}`"
        /></label>
      </div>
      <footer>
        <button type="button" @click="close">取消</button
        ><button type="button" class="primary" :disabled="!valid" @click="confirm">确定</button>
      </footer>
    </dialog>
  </div>
</template>
<style scoped>
.time-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}
.time-field > label {
  color: var(--text-muted);
  font-size: 10px;
}
.time-control {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 3px;
  padding: 4px 3px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel);
}
.time-control > input {
  flex: 1 1 0;
  width: 0;
  min-width: 0;
  padding: 3px 0;
  border: 0;
  background: transparent;
  font-size: 10px;
}
.time-control > input::-webkit-calendar-picker-indicator {
  display: none;
}
.time-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex: 0 0 auto;
}
.time-actions button {
  display: inline-grid;
  place-items: center;
  min-height: 23px;
  min-height: 23px;
  padding: 1px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--accent);
  font-size: 9px;
}
.time-actions .ui-icon {
  width: 12px;
  height: 12px;
}
.time-popover {
  position: fixed;
  inset: 0;
  width: min(300px, calc(100% - 20px));
  max-height: calc(100% - 24px);
  margin: auto;
  padding: 12px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel);
  color: var(--text);
  box-shadow: 0 12px 36px #20301d2b;
}
.time-popover::backdrop {
  background: #21331f28;
}
.time-popover header {
  font-weight: 700;
  margin-bottom: 12px;
}
.time-parts {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}
.time-parts label {
  display: flex;
  align-items: center;
  gap: 3px;
  min-width: 0;
  font-size: 10px;
  color: var(--text-muted);
}
.time-parts input {
  width: 0;
  min-width: 0;
  flex: 1;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--panel);
  padding: 5px 3px;
  font-size: 11px;
}
.time-popover footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}
.time-popover footer button {
  padding: 6px 10px;
  border-radius: 7px;
}
</style>
