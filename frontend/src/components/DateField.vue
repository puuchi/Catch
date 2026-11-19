<script setup lang="ts">
import { nextTick, ref, useId, watch } from 'vue';
import Icon from './Icon.vue';
import CalendarDialog from './CalendarDialog.vue';
import { parseDate } from '../calendar';
const value = defineModel<string>({ required: true });
withDefaults(defineProps<{ label?: string; placeholder?: string; clearable?: boolean }>(), {
  label: '日期',
  placeholder: '选择日期',
  clearable: true,
});
const input = ref<HTMLInputElement>();
const id = useId();
const opened = ref(false),
  draft = ref(value.value),
  invalid = ref(false);
watch(value, (v) => {
  draft.value = v;
  invalid.value = false;
});
function commit() {
  if (!draft.value || parseDate(draft.value)) {
    value.value = draft.value;
    invalid.value = false;
  } else invalid.value = true;
}
function typing(event: Event) {
  draft.value = (event.target as HTMLInputElement).value;
  invalid.value = false;
  if (!draft.value || parseDate(draft.value)) value.value = draft.value;
}
async function close() {
  opened.value = false;
  await nextTick();
  input.value?.focus({ preventScroll: true });
}
</script>
<template>
  <div class="date-field">
    <label :for="id">{{ label }}</label>
    <div class="date-control">
      <input
        :id="id"
        ref="input"
        v-model="draft"
        type="text"
        inputmode="numeric"
        maxlength="10"
        :placeholder="placeholder"
        :aria-label="label"
        :aria-invalid="invalid || undefined"
        @input="typing"
        @change="commit"
        @keydown.enter.prevent="commit"
        @keydown.arrow-down.prevent="opened = true"
      />
      <button
        type="button"
        :aria-label="`选择${label}`"
        v-tooltip="`选择${label}`"
        @click="opened = true"
      >
        <Icon name="calendar" />
      </button>
    </div>
    <span v-if="invalid" class="date-error" role="alert">请输入有效日期，如 2026-11-18</span>
    <CalendarDialog
      v-if="opened"
      :value="value"
      :label="label"
      :clearable="clearable"
      @select="
        value = $event;
        draft = $event;
        invalid = false;
      "
      @close="close"
    />
  </div>
</template>
<style scoped>
.date-field {
  min-width: 0;
  width: 100%;
}
.date-field > label {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
.date-control {
  display: flex;
  align-items: center;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel);
}
.date-control input {
  width: 0;
  min-width: 0;
  flex: 1;
  padding: 7px 6px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.date-control button {
  display: grid;
  place-items: center;
  flex: 0 0 25px;
  width: 25px;
  height: 29px;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
}
</style>
