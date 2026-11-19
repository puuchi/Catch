<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
defineProps<{ message: string }>();
const emit = defineEmits<{ dismiss: [] }>();
const root = ref<HTMLElement>();
const dismiss = (event: PointerEvent) => {
  if (!root.value?.parentElement?.contains(event.target as Node)) emit('dismiss');
};
const key = (event: KeyboardEvent) => {
  if (event.key === 'Escape') emit('dismiss');
};
onMounted(() => {
  document.addEventListener('pointerdown', dismiss, true);
  document.addEventListener('keydown', key);
});
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', dismiss, true);
  document.removeEventListener('keydown', key);
});
</script>
<template>
  <span ref="root" v-if="message" class="validation-hint" role="alert">{{ message }}</span>
</template>
