<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
import Icon from './Icon.vue';
defineProps<{ title: string }>();
const emit = defineEmits<{ close: [] }>();
const dialog = ref<HTMLDialogElement>();
onMounted(() => dialog.value?.showModal());
onBeforeUnmount(() => dialog.value?.close());
</script>
<template>
  <dialog
    ref="dialog"
    class="modal"
    @cancel.prevent="emit('close')"
    @click="$event.target === dialog && emit('close')"
  >
    <div class="modal-card">
      <header class="editor-heading">
        <strong>{{ title }}</strong><button type="button" aria-label="关闭" @click="emit('close')">
          <Icon name="close" />
        </button>
      </header>
      <slot />
    </div>
  </dialog>
</template>
