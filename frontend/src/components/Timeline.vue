<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import Icon from './Icon.vue';
import DateField from './DateField.vue';
import Resources from './Resources.vue';
import ActionMenu from './ActionMenu.vue';
import EffortEntry from './EffortEntry.vue';
import TimelineGap from './TimelineGap.vue';
import { busy, dispatch, editMatter, editor, inform, now, state, ui } from '../store';
import { bounds, day, duration, human, inputTime, statusShort } from '../display';
import type { Entry } from '../display';
import {
  detailGroups,
  focusRows,
  focusSummary,
  HEAT_LEVELS,
  sessionKey,
  timelineRows,
} from '../timeline';
import { openSessionEditor } from '../session-editor';
import { useScroll } from '../scroll';
const timelineScroll = ref<HTMLElement>();
const detailsScroll = ref<HTMLElement>();
// Focus is a presentation change within the same day, not a new scroll context.
const saveTimelineScroll = useScroll(timelineScroll, () => `timeline:${ui.timelineDate}`);
const saveDetailsScroll = useScroll(detailsScroll, () => `matter:${ui.selectedMatterId}`);
const activities = computed(() => state.value?.tracking.activities || []);
const details = computed(() =>
  activities.value.find((matter) => matter.id === ui.selectedMatterId),
);
const groups = computed(() => (details.value ? detailGroups(details.value, now.value) : []));
const originDate = computed(() => {
  const containing = groups.value.filter((group) =>
    group.entries.some((entry) => entry.session.id === ui.selectedSessionId),
  );
  return (
    containing.find((group) => group.date === ui.selectedSessionDate)?.date || containing[0]?.date
  );
});
const legend = ref(false);
const menu = ref<{
  entry: Entry;
  date: string;
  x: number;
  y: number;
  trigger?: HTMLElement;
} | null>(null);
const allRows = computed(() => timelineRows(activities.value, ui.timelineDate, now.value));
const focused = computed(() => activities.value.find((matter) => matter.id === ui.focusedMatterId));
const rows = computed(() =>
  focused.value
    ? focusRows(allRows.value, focused.value.id, ui.timelineFocusMode !== 'soft')
    : allRows.value,
);
const summary = computed(() =>
  focused.value ? focusSummary(focused.value, ui.timelineDate, now.value) : null,
);
const menuItems = computed(() => {
  const target = menu.value;
  if (!target) return [];
  return [
    ...(!details.value ? [{ id: 'details', label: '查看详情', icon: 'arrow' }] : []),
    { id: 'edit', label: '编辑本次', icon: 'edit', disabled: !!busy.value },
    { id: 'delete', label: '删除本次', icon: 'trash', danger: true, disabled: !!busy.value },
  ];
});
function clearFocus() {
  ui.focusedMatterId = null;
  ui.timelineFocusMode = undefined;
}
function focus(entry: Entry, only: boolean) {
  if (!only && ui.focusedMatterId === entry.matter.id && ui.timelineFocusMode === 'soft')
    clearFocus();
  else {
    ui.focusedMatterId = entry.matter.id;
    ui.timelineFocusMode = only ? 'only' : 'soft';
  }
}
function move(n: number) {
  const date = new Date(`${ui.timelineDate}T12:00:00`);
  date.setDate(date.getDate() + n);
  ui.timelineDate = day(+date);
}
function record(activityId?: string, start?: number, end?: number) {
  if (busy.value) return;
  const stop = end ?? Math.min(now.value, bounds(ui.timelineDate)[1] - 1);
  const from = start ?? Math.max(bounds(ui.timelineDate)[0], stop - 30 * 60_000);
  ui.editor = editor({
    kind: 'record',
    activityId,
    start: inputTime(new Date(from).toISOString()),
    end: inputTime(new Date(stop).toISOString()),
  });
}
function showMenu(event: MouseEvent, entry: Entry, date: string) {
  event.preventDefault();
  const element = event.currentTarget as HTMLElement;
  const trigger = element.matches('button')
    ? element
    : element.querySelector<HTMLElement>('.effort-menu') || element;
  const rect = trigger.getBoundingClientRect();
  menu.value = {
    entry,
    date,
    x: event.type === 'contextmenu' ? event.clientX : rect.left,
    y: event.type === 'contextmenu' ? event.clientY : rect.bottom,
    trigger,
  };
}
async function menuAction(id: string) {
  const target = menu.value;
  menu.value = null;
  if (!target) return;
  const { entry, date } = target;
  if (id === 'details') {
    ui.selectedSessionId = entry.session.id;
    ui.selectedSessionDate = date;
    ui.selectedMatterId = entry.matter.id;
  } else if (id === 'edit') await openSessionEditor(entry.matter, entry.session);
  else if (id === 'delete')
    await remove('delete-session', { activityId: entry.matter.id, sessionId: entry.session.id });
}
async function remove(type: string, payload: Record<string, unknown>) {
  if (busy.value) return;
  try {
    await dispatch('tracking', type, payload);
    if (type === 'remove') ui.selectedMatterId = null;
  } catch (error) {
    inform(error);
  }
}
watch(
  () => [ui.selectedMatterId, ui.selectedSessionId, originDate.value],
  async () => {
    if (!details.value || !ui.selectedSessionId || !originDate.value) return;
    await nextTick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    const key = sessionKey(ui.selectedSessionId, originDate.value);
    detailsScroll.value
      ?.querySelector<HTMLElement>(`[data-session-key="${CSS.escape(key)}"]`)
      ?.scrollIntoView({ block: 'center' });
  },
  { immediate: true },
);
watch(activities, () => {
  if (ui.focusedMatterId && !focused.value) clearFocus();
});
</script>
<template>
  <section class="module-page timeline-page" aria-label="时间轴">
    <template v-if="details">
      <div class="details-fixed">
        <header class="details-heading">
          <button
            class="icon-button"
            v-tooltip="'返回时间轴'"
            aria-label="返回时间轴"
            @click="ui.selectedMatterId = null"
          >
            <Icon name="back" />
          </button>
          <span class="spacer" />
          <button
            class="icon-button"
            v-tooltip="'编辑事项'"
            aria-label="编辑事项"
            :disabled="!!busy"
            @click="editMatter(details.id)"
          >
            <Icon name="edit" />
          </button>
          <button
            class="icon-button"
            aria-label="补录一次投入"
            v-tooltip="'补录一次投入'"
            :disabled="!!busy"
            @click="record(details.id)"
          >
            <Icon name="plus" />
          </button>
          <button
            class="icon-button"
            v-tooltip="'删除整个事项'"
            aria-label="删除整个事项"
            :disabled="!!busy"
            @click="remove('remove', { id: details.id })"
          >
            <Icon name="trash" />
          </button>
        </header>
        <div class="matter-title">
          <i class="status-dot" :style="{ background: details.color }" />
          <h2>{{ details.name }}</h2>
          <span class="timeline-status">{{ statusShort(details.status) }}</span>
        </div>
      </div>
      <div ref="detailsScroll" class="matter-detail-body" @scroll="saveDetailsScroll">
        <div class="matter-stats">
          <div>
            <strong>{{ human(duration(details, now)) }}</strong><span>累计投入</span>
          </div>
          <div>
            <strong>{{ details.sessions.length }} 次</strong><span>投入</span>
          </div>
        </div>
        <Resources :owner="{ kind: 'matter', id: details.id }" :resources="details.resources" />
        <section
          v-for="group in groups"
          :key="group.date"
          class="detail-day"
          :aria-label="group.date"
        >
          <h3>{{ group.date.replaceAll('-', '/') }}</h3>
          <EffortEntry
            v-for="entry in group.entries"
            :key="sessionKey(entry.session.id, group.date)"
            :entry="entry"
            :date="group.date"
            detail
            :origin="ui.selectedSessionId === entry.session.id && originDate === group.date"
            @menu="showMenu($event, entry, group.date)"
          />
        </section>
      </div>
    </template>
    <template v-else>
      <div class="timeline-toolbar">
        <aside v-if="focused && summary" class="timeline-focus-summary" aria-label="当日事项摘要">
          <div>
            <i class="status-dot" :style="{ background: focused.color }" />
            <strong v-tooltip="focused.name">{{ focused.name }}</strong>
            <button
              class="icon-button"
              v-tooltip="'取消聚焦'"
              aria-label="取消聚焦"
              @click="clearFocus"
            >
              <Icon name="close" />
            </button>
          </div>
          <p>当日 {{ summary.count }} 次投入 · 累计 {{ human(summary.duration) }}</p>
        </aside>
        <div v-else class="timeline-date-controls">
          <button class="date-nav" aria-label="前一天" v-tooltip="'前一天'" @click="move(-1)">
            ‹
          </button>
          <DateField v-model="ui.timelineDate" label="时间轴日期" />
          <button class="date-nav" aria-label="后一天" v-tooltip="'后一天'" @click="move(1)">
            ›
          </button>
          <button class="date-nav today-button" @click="ui.timelineDate = day(now)">今天</button>
        </div>
        <Teleport defer to="#timeline-tools">
          <button
            class="icon-button"
            v-tooltip="'颜色说明'"
            aria-label="颜色说明"
            :aria-expanded="legend"
            @click="legend = !legend"
          >
            <Icon name="info" />
          </button>
          <button
            class="icon-button"
            v-tooltip="'补录'"
            aria-label="补录"
            :disabled="!!busy"
            @click="record()"
          >
            <Icon name="plus" />
          </button>
        </Teleport>
      </div>
      <section v-if="legend" class="heat-legend" aria-label="颜色说明">
        <p>颜色区分事项，深浅表示时长</p>
        <div class="heat-levels">
          <span v-for="level in HEAT_LEVELS" :key="level.label"
            ><i :style="{ background: `rgba(46,90,73,${level.opacity})` }" />{{ level.label }}</span
          >
        </div>
      </section>
      <div ref="timelineScroll" class="timeline-list" @scroll="saveTimelineScroll">
        <template v-for="row in rows" :key="row.key">
          <EffortEntry
            v-if="row.kind === 'entry'"
            :entry="row.entry"
            :date="ui.timelineDate"
            :dimmed="!!focused && focused.id !== row.entry.matter.id"
            :focused="focused?.id === row.entry.matter.id"
            @focus="focus(row.entry, $event)"
            @menu="showMenu($event, row.entry, ui.timelineDate)"
          />
          <TimelineGap
            v-else
            :row="row"
            :disabled="!!busy"
            @record="record(undefined, row.start, row.end)"
          />
        </template>
        <p v-if="!rows.length" class="empty-state">这一天，还没有投入记录。</p>
      </div>
    </template>
    <ActionMenu
      v-if="menu"
      :items="menuItems"
      :x="menu.x"
      :y="menu.y"
      :trigger="menu.trigger"
      label="投入操作"
      @select="menuAction"
      @close="menu = null"
    />
  </section>
</template>
<style scoped>
.timeline-page {
  overflow: hidden;
  position: relative;
}
.timeline-toolbar {
  display: flex;
  align-items: center;
  flex: 0 0 51px;
  height: 51px;
  box-sizing: border-box;
  padding: 8px 12px 6px;
}
.timeline-date-controls {
  display: grid;
  grid-template-columns: 24px minmax(95px, 1fr) 24px auto;
  align-items: center;
  gap: 4px;
  flex: 1 1 200px;
  min-width: 0;
}
.date-nav {
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--panel);
  min-width: 24px;
  height: 29px;
  padding: 0 5px;
  font-size: 11px;
  color: var(--text);
}
.timeline-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 12px 10px 8px;
  min-width: 0;
}
.heat-legend {
  position: absolute;
  z-index: 20;
  left: 12px;
  right: 12px;
  bottom: 8px;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--panel);
  box-shadow: 0 5px 18px #25332d18;
  color: var(--text-muted);
  flex: 0 0 auto;
  font-size: 9px;
}
.heat-legend p {
  margin-bottom: 6px;
}
.heat-levels {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 5px 8px;
}
.heat-levels > span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 9px;
  white-space: nowrap;
}
.heat-levels i {
  width: 10px;
  height: 10px;
  border-radius: 2px;
}
.details-fixed {
  flex: 0 0 auto;
  padding: 0 12px 8px;
  border-bottom: 1px solid var(--border);
}
.details-heading {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding: 5px 0;
}
.matter-title {
  display: flex;
  gap: 7px;
  justify-content: space-between;
  align-items: center;
}
.matter-title h2 {
  flex: 1;
  min-width: 0;
  line-height: 1.4;
  font-size: 16px;
  overflow-wrap: anywhere;
}
.matter-title .timeline-status {
  flex: 0 0 auto;
  font-size: 9px;
  color: var(--text-muted);
  white-space: nowrap;
}
.matter-stats {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(0, 1fr);
  gap: 0;
  padding: 12px 5px;
  margin: 6px 4px 12px;
  border-radius: 12px;
  background: #eaf0e180;
  text-align: center;
}
.matter-stats > div {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  justify-content: center;
}
.matter-stats > div + div {
  border-left: 1px solid var(--border);
}
.matter-stats strong {
  overflow-wrap: anywhere;
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
}
.matter-stats span {
  font-size: 9px;
  color: var(--text-muted);
}
.matter-detail-body {
  flex: 1;
  min-height: 0;
  min-width: 0;
  overflow-y: auto;
  padding: 4px 12px 12px 8px;
}
.detail-day h3 {
  font-size: 10px;
  color: var(--text-muted);
  padding: 10px 0 6px;
  font-weight: 600;
  border-bottom: 1px solid var(--border);
}
.timeline-focus-summary {
  flex: 1;
  min-width: 0;
  align-self: stretch;
  display: flex;
  flex-direction: column;
  justify-content: center;
  background: #e2e9d9;
  border: 1px solid #c5d2bb;
  border-radius: 8px;
  padding: 0 7px;
}
.timeline-focus-summary > div {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 20px;
}
.timeline-focus-summary strong {
  min-width: 0;
  flex: 1;
  font-size: 11px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.timeline-focus-summary p {
  padding-left: 12px;
  font-size: 10px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
