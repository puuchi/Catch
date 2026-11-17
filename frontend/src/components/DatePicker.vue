<script setup lang="ts">
import { ref, useId } from 'vue';
import Icon from './Icon.vue';
defineProps<{ modelValue: string | null }>();
const emit = defineEmits<{ 'update:modelValue': [value: string | null] }>();
const input = ref<HTMLInputElement>();
const tooltipId = useId();
</script>
<template>
  <span class="date-option">
    <button
      type="button"
      class="icon-button"
      aria-label="截止日期"
      :aria-describedby="tooltipId"
      @click="input?.showPicker()"
    >
      <Icon name="calendar" />
    </button>
    <input
      ref="input"
      type="date"
      :value="modelValue || ''"
      tabindex="-1"
      title="截止日期"
      aria-label="截止日期"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).value || null)"
    />
    <span :id="tooltipId" class="date-tooltip" role="tooltip">截止日期</span>
  </span>
</template>
