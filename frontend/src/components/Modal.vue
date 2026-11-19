<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, useId } from 'vue';
import Icon from './Icon.vue';
defineProps<{ title: string }>();
const emit = defineEmits<{ close: [] }>();
const dialog = ref<HTMLDialogElement>();
const titleId = useId();
onMounted(() => dialog.value?.showModal());
onBeforeUnmount(() => dialog.value?.close());
</script>
<template>
  <dialog
    ref="dialog"
    class="modal"
    :aria-labelledby="titleId"
    @cancel.prevent="emit('close')"
    @click="$event.target === dialog && emit('close')"
  >
    <div class="modal-card">
      <header class="editor-heading">
        <strong :id="titleId">{{ title }}</strong><button
          type="button"
          class="icon-button"
          v-tooltip="'关闭'"
          aria-label="关闭"
          @click="emit('close')"
        >
          <Icon name="close" />
        </button>
      </header>
      <slot />
    </div>
  </dialog>
</template>
