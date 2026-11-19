<script setup lang="ts">
import { nextTick, ref } from 'vue';
import Icon from './Icon.vue';
import CalendarDialog from './CalendarDialog.vue';
defineProps<{ modelValue: string | null }>();
const emit = defineEmits<{ 'update:modelValue': [value: string | null] }>();
const opened = ref(false),
  trigger = ref<HTMLButtonElement>();
async function close() {
  opened.value = false;
  await nextTick();
  trigger.value?.focus({ preventScroll: true });
}
</script>
<template>
  <span class="date-option">
    <button
      ref="trigger"
      type="button"
      class="icon-button"
      :class="{ active: !!modelValue }"
      aria-label="截止日期"
      v-tooltip="modelValue ? `截止日期 ${modelValue}` : '截止日期'"
      @click="opened = true"
    >
      <Icon name="calendar" />
    </button>
    <CalendarDialog
      v-if="opened"
      :value="modelValue || ''"
      label="截止日期"
      clearable
      @select="emit('update:modelValue', $event || null)"
      @close="close"
    />
  </span>
</template>
