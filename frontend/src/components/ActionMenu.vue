<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import Icon from './Icon.vue';

const props = defineProps<{
  items: { id: string; label: string; icon: string; disabled?: boolean; danger?: boolean }[];
  x: number;
  y: number;
  trigger?: HTMLElement;
  label: string;
}>();
const emit = defineEmits<{ select: [id: string]; close: [] }>();
const menu = ref<HTMLElement>();
const position = ref({ left: `${props.x}px`, top: `${props.y}px` });

function close(restoreFocus = false) {
  if (restoreFocus && props.trigger?.isConnected) props.trigger.focus({ preventScroll: true });
  emit('close');
}
function outside(event: PointerEvent) {
  if (!menu.value?.contains(event.target as Node)) close();
}
function key(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    event.stopPropagation();
    close(true);
    return;
  }
  if (event.key === 'Tab') {
    close(true);
    return;
  }
  if (!['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key))
    return;
  event.preventDefault();
  const buttons = Array.from(
    menu.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') || [],
  );
  if (!buttons.length) return;
  const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
  let index = current;
  if (event.key === 'Home') index = 0;
  else if (event.key === 'End') index = buttons.length - 1;
  else {
    const direction = ['ArrowDown', 'ArrowRight'].includes(event.key) ? 1 : -1;
    index = (current + direction + buttons.length) % buttons.length;
  }
  buttons[index]?.focus();
}
function dismiss() {
  close();
}
onMounted(async () => {
  await nextTick();
  const rect = menu.value?.getBoundingClientRect();
  if (rect) {
    position.value = {
      left: `${Math.max(8, Math.min(props.x, window.innerWidth - rect.width - 8))}px`,
      top: `${Math.max(8, Math.min(props.y, window.innerHeight - rect.height - 8))}px`,
    };
  }
  menu.value
    ?.querySelector<HTMLButtonElement>('button:not(:disabled)')
    ?.focus({ preventScroll: true });
  document.addEventListener('pointerdown', outside);
  window.addEventListener('resize', dismiss);
  window.addEventListener('scroll', dismiss, true);
});
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', outside);
  window.removeEventListener('resize', dismiss);
  window.removeEventListener('scroll', dismiss, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      ref="menu"
      class="action-menu"
      role="menu"
      :aria-label="label"
      :style="position"
      @keydown="key"
      @contextmenu.prevent
    >
      <button
        v-for="item in items"
        :key="item.id"
        type="button"
        class="icon-button"
        :class="{ 'action-menu-danger': item.danger }"
        role="menuitem"
        :title="item.label"
        :aria-label="item.label"
        :disabled="item.disabled"
        @click="
          emit('select', item.id);
          emit('close');
        "
      >
        <Icon :name="item.icon" />
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.action-menu {
  position: fixed;
  z-index: 100;
  display: flex;
  gap: 3px;
  padding: 5px;
  border: 1px solid #d6dfc9;
  border-radius: 9px;
  background: #fffef9;
  box-shadow: 0 7px 20px #25332d20;
}
.action-menu .icon-button {
  width: 30px;
  height: 30px;
}
.action-menu-danger {
  color: var(--urgent);
}
</style>
