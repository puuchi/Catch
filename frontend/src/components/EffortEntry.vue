<script setup lang="ts">
import { computed } from 'vue';
import Icon from './Icon.vue';
import { day, human, time } from '../display';
import type { Entry } from '../display';
import { effortTint, isContinuation, sessionKey } from '../timeline';
const props = defineProps<{
  entry: Entry;
  date: string;
  dimmed?: boolean;
  focused?: boolean;
  origin?: boolean;
  detail?: boolean;
}>();
const emit = defineEmits<{ focus: [only: boolean]; menu: [event: MouseEvent] }>();
const caption = computed(() => {
  if (props.entry.session.progress) return props.entry.session.progress;
  if (!props.entry.session.endTime) return '进行中';
  if (
    props.entry.matter.sessions.at(-1)?.id === props.entry.session.id &&
    props.entry.matter.status === 'paused'
  )
    return '已暂停';
  return '';
});
function click(event: MouseEvent) {
  if (!props.detail && event.detail <= 1) emit('focus', false);
}
function key(event: KeyboardEvent) {
  if (event.target !== event.currentTarget) return;
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    if (!props.detail) emit('focus', event.shiftKey);
  }
  if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
    event.preventDefault();
    const trigger = (event.currentTarget as HTMLElement).querySelector<HTMLButtonElement>(
      '.effort-menu',
    );
    trigger?.click();
  }
}
</script>
<template>
  <article
    class="effort-entry"
    :class="{ 'effort-dimmed': dimmed, 'effort-focused': focused, 'effort-origin': origin }"
    :style="effortTint(entry.matter.color, entry.duration)"
    :data-session-key="sessionKey(entry.session.id, date)"
    :data-session-id="entry.session.id"
    :aria-label="`${entry.matter.name}，${human(entry.duration)}`"
    :aria-current="origin ? 'true' : undefined"
    tabindex="0"
    @click="click"
    @dblclick="!detail && emit('focus', true)"
    @keydown="key"
    @contextmenu.prevent="emit('menu', $event)"
  >
    <div class="effort-times">
      <time :datetime="new Date(entry.start).toISOString()">{{
        time(new Date(entry.start).toISOString())
      }}</time>
      <time :datetime="new Date(entry.end).toISOString()"
        ><small v-if="day(entry.end) !== date">次日</small
        >{{
          !entry.session.endTime && day(entry.end) === date
            ? '现在'
            : time(new Date(entry.end).toISOString())
        }}</time
      >
    </div>
    <i class="effort-bar" aria-hidden="true" />
    <div class="effort-content">
      <div class="effort-heading">
        <span class="effort-name" :title="entry.matter.name"
          ><span v-if="isContinuation(entry)" class="effort-return" aria-label="再次投入">↩</span
          >{{ entry.matter.name }}</span
        >
        <span class="effort-duration">{{ human(entry.duration) }}</span>
      </div>
      <div class="effort-meta">
        <p class="effort-progress">{{ caption }}</p>
        <button
          class="icon-button effort-menu"
          :aria-label="`投入操作：${entry.matter.name}`"
          title="投入操作"
          aria-haspopup="menu"
          @click.stop="emit('menu', $event)"
          @dblclick.stop
        >
          <Icon name="more" />
        </button>
      </div>
      <p v-if="origin" class="effort-source">从这次投入进入</p>
    </div>
  </article>
</template>
<style scoped>
.effort-entry {
  display: grid;
  grid-template-columns: 30px 3px minmax(0, 1fr);
  gap: 5px;
  padding: 9px 0;
  min-width: 0;
  border-bottom: 1px solid var(--border);
  transition: opacity 0.16s;
  cursor: pointer;
}
.effort-times {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: flex-end;
  color: var(--text-done);
  font-size: 9px;
  padding: 2px 0;
  white-space: nowrap;
}
.effort-times time {
  text-align: right;
}
.effort-times small {
  display: block;
  font-size: 8px;
}
.effort-bar {
  border-radius: 2px;
  background: var(--effort-bar);
  min-height: 42px;
}
.effort-content {
  min-width: 0;
  background: var(--effort-wash);
  border-radius: 6px;
  padding: 5px 6px;
}
.effort-heading {
  display: flex;
  align-items: flex-start;
  gap: 5px;
}
.effort-name {
  min-width: 0;
  flex: 1;
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.effort-return {
  color: var(--text-muted);
  margin-right: 4px;
}
.effort-duration {
  flex: 0 0 auto;
  font-size: 10px;
  white-space: nowrap;
  color: color-mix(in srgb, var(--effort-color) 55%, var(--text));
}
.effort-meta {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  min-height: 22px;
}
.effort-progress {
  flex: 1;
  min-width: 0;
  font-size: 10px;
  color: var(--text-muted);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
  margin-top: 3px;
}
.effort-menu {
  width: 21px;
  height: 21px;
  margin: 0 0 -2px;
}
.effort-dimmed {
  opacity: 0.3;
}
.effort-focused .effort-content {
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--effort-color) 28%, transparent);
}
.effort-origin .effort-content {
  box-shadow: inset 0 0 0 1px var(--effort-bar);
}
.effort-source {
  color: var(--accent);
  font-size: 9px;
  margin-top: 3px;
}
@media (prefers-reduced-motion: reduce) {
  .effort-entry {
    transition: none;
  }
}
</style>
