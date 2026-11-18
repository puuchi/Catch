<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue';
import { calendarDays, dateValue, parseDate } from '../calendar';
import { now } from '../store';
import Icon from './Icon.vue';
const props = defineProps<{ value: string; label: string; clearable?: boolean }>();
const emit = defineEmits<{ select: [value: string]; close: [] }>();
const dialog = ref<HTMLDialogElement>();
const initial = parseDate(props.value) || new Date(now.value);
const year = ref(initial.getFullYear()),
  month = ref(initial.getMonth());
const active = ref(dateValue(initial));
const days = computed(() => calendarDays(year.value, month.value));
const today = computed(() => dateValue(new Date(now.value)));
function move(amount: number) {
  const date = new Date(year.value, month.value + amount, 1, 12);
  if (date.getFullYear() < 1 || date.getFullYear() > 9999) return;
  year.value = date.getFullYear();
  month.value = date.getMonth();
}
function select(value: string) {
  emit('select', value);
  emit('close');
}
async function keyboard(event: KeyboardEvent, value: string) {
  const delta: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 };
  if (!(event.key in delta) && !['Home', 'End', 'PageUp', 'PageDown'].includes(event.key)) return;
  event.preventDefault();
  const date = parseDate(value)!;
  if (event.key === 'PageUp' || event.key === 'PageDown')
    date.setMonth(date.getMonth() + (event.key === 'PageUp' ? -1 : 1));
  else if (event.key === 'Home') date.setDate(date.getDate() - ((date.getDay() + 6) % 7));
  else if (event.key === 'End') date.setDate(date.getDate() + 6 - ((date.getDay() + 6) % 7));
  else date.setDate(date.getDate() + delta[event.key]);
  if (date.getFullYear() < 1 || date.getFullYear() > 9999) return;
  active.value = dateValue(date);
  year.value = date.getFullYear();
  month.value = date.getMonth();
  await nextTick();
  dialog.value?.querySelector<HTMLButtonElement>(`[data-date="${active.value}"]`)?.focus();
}
onMounted(async () => {
  dialog.value?.showModal();
  await nextTick();
  dialog.value?.querySelector<HTMLButtonElement>(`[data-date="${active.value}"]`)?.focus();
});
</script>
<template>
  <dialog
    ref="dialog"
    class="calendar-dialog"
    :aria-label="`选择${label}`"
    @cancel.prevent.stop="emit('close')"
    @keydown.stop
    @click.stop="$event.target === dialog && emit('close')"
  >
    <header class="calendar-heading">
      <span>{{ label }}</span><button
        type="button"
        class="icon-button"
        aria-label="关闭日历"
        @click="emit('close')"
      >
        <Icon name="close" />
      </button>
    </header>
    <div class="calendar-month">
      <button type="button" class="icon-button" aria-label="上个月" @click="move(-1)">
        <Icon name="back" />
      </button>
      <label
        ><input
          v-model.number="year"
          type="number"
          min="1"
          max="9999"
          aria-label="年份"
          @change="year = Math.max(1, Math.min(9999, Number(year) || initial.getFullYear()))"
        />年</label
      >
      <select v-model.number="month" aria-label="月份">
        <option v-for="n in 12" :key="n" :value="n - 1">{{ n }}月</option>
      </select>
      <button
        type="button"
        class="icon-button calendar-forward"
        aria-label="下个月"
        @click="move(1)"
      >
        <Icon name="back" />
      </button>
    </div>
    <div class="calendar-week" aria-hidden="true">
      <span v-for="name in ['一', '二', '三', '四', '五', '六', '日']" :key="name">{{ name }}</span>
    </div>
    <div class="calendar-days" role="group" aria-label="选择日期">
      <button
        v-for="date in days"
        :key="date.value"
        type="button"
        :data-date="date.value"
        :aria-label="date.value"
        :aria-pressed="date.value === value"
        :aria-current="date.value === today ? 'date' : undefined"
        :tabindex="date.value === active ? 0 : -1"
        :class="{
          outside: date.outside,
          selected: date.value === value,
          today: date.value === today,
        }"
        @focus="active = date.value"
        @keydown="keyboard($event, date.value)"
        @click="select(date.value)"
      >
        {{ date.day }}
      </button>
    </div>
    <footer>
      <button v-if="clearable" type="button" @click="select('')">清除</button
      ><span class="spacer" /><button type="button" @click="select(today)">今天</button
      ><button type="button" @click="emit('close')">取消</button>
    </footer>
  </dialog>
</template>
<style scoped>
.calendar-dialog {
  position: fixed;
  inset: 0;
  margin: auto;
  width: min(274px, calc(100% - 20px));
  max-height: calc(100% - 20px);
  overflow: auto;
  padding: 12px;
  border: 1px solid #ccd7c2;
  border-radius: 14px;
  background: #fafbf5;
  color: var(--text);
  box-shadow: 0 12px 40px #20301d30;
}
.calendar-dialog::backdrop {
  background: #21331f35;
  backdrop-filter: blur(3px);
}
.calendar-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 600;
  margin-bottom: 8px;
}
.calendar-month {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
  margin-bottom: 10px;
}
.calendar-month label {
  display: flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
}
.calendar-month input {
  width: 51px;
  padding: 4px 2px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--panel);
  font-size: 11px;
  text-align: center;
  appearance: textfield;
}
.calendar-month input::-webkit-inner-spin-button {
  appearance: none;
}
.calendar-month select {
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--panel);
  font-size: 11px;
}
.calendar-forward {
  transform: rotate(180deg);
}
.calendar-week,
.calendar-days {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 3px;
  text-align: center;
}
.calendar-week {
  font-size: 9px;
  color: var(--text-muted);
  margin-bottom: 5px;
}
.calendar-days button {
  min-width: 0;
  height: 28px;
  padding: 0;
  border-radius: 7px;
  font-size: 11px;
  color: var(--text);
}
.calendar-days button:hover {
  background: #e2ead9;
}
.calendar-days .outside {
  color: #a6af9e;
}
.calendar-days .today {
  box-shadow: inset 0 0 0 1px #9cb58c;
}
.calendar-days .selected {
  background: var(--accent);
  color: white;
}
.calendar-days button:focus-visible {
  outline: 2px solid #64825d;
  outline-offset: 1px;
}
.calendar-dialog footer {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
.calendar-dialog footer button {
  padding: 4px 2px;
  font-size: 10px;
  color: var(--accent);
}
</style>
