<script setup lang="ts">
import { computed, ref } from 'vue';
import Icon from './Icon.vue';
import Resources from './Resources.vue';
import { busy, dispatch, editMatter, editor, inform, now, state, ui } from '../store';
import {
  bounds,
  day,
  duration,
  entries,
  gaps,
  human,
  inputTime,
  statusShort,
  time,
} from '../display';
import type { Matter, Session } from '../models';
import { useScroll } from '../scroll';
const timelineScroll = ref<HTMLElement>(),
  detailsScroll = ref<HTMLElement>();
const saveTimelineScroll = useScroll(
  timelineScroll,
  () => `timeline:${ui.timelineDate}:${ui.focusedMatterId || 'all'}`,
);
const saveDetailsScroll = useScroll(detailsScroll, () => `matter:${ui.selectedMatterId}`);
const details = computed(() =>
  state.value?.tracking.activities.find((m) => m.id === ui.selectedMatterId),
);
const legend = ref(false),
  menu = ref<string | null>(null);
const list = computed(() =>
  entries(state.value?.tracking.activities || [], ui.timelineDate, now.value).filter(
    (e) => !ui.focusedMatterId || e.matter.id === ui.focusedMatterId,
  ),
);
const blanks = computed(() =>
  ui.focusedMatterId
    ? []
    : gaps(state.value?.tracking.activities || [], ui.timelineDate, now.value),
);
const total = computed(
  () =>
    state.value?.tracking.activities.reduce(
      (sum, m) => sum + duration(m, now.value, bounds(ui.timelineDate)),
      0,
    ) || 0,
);
function move(n: number) {
  const date = new Date(`${ui.timelineDate}T12:00:00`);
  date.setDate(date.getDate() + n);
  ui.timelineDate = day(+date);
}
function record(activityId?: string, start?: number, end?: number) {
  const stop = end ?? Math.min(now.value, bounds(ui.timelineDate)[1] - 1),
    from = start ?? Math.max(bounds(ui.timelineDate)[0], stop - 30 * 60_000);
  ui.editor = editor({
    kind: 'record',
    activityId,
    start: inputTime(new Date(from).toISOString()),
    end: inputTime(new Date(stop).toISOString()),
  });
}
function editSession(matter: Matter, session: Session) {
  ui.editor = editor({
    kind: 'session',
    activityId: matter.id,
    sessionId: session.id,
    name: matter.name,
    progress: session.progress,
    start: inputTime(session.startTime),
    end: inputTime(session.endTime || new Date(now.value).toISOString()),
    original: JSON.parse(JSON.stringify(session)),
  });
}
async function remove(type: string, payload: Record<string, unknown>) {
  try {
    await dispatch('tracking', type, payload);
    menu.value = null;
    if (type === 'remove') ui.selectedMatterId = null;
  } catch (error) {
    inform(error);
  }
}
function tint(color: string, ms: number) {
  const rgb = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16)).join(',');
  return {
    borderLeftColor: color,
    backgroundColor: `rgba(${rgb},${ms >= 3_600_000 ? 0.15 : ms >= 1_800_000 ? 0.11 : ms >= 900_000 ? 0.075 : 0.045})`,
  };
}
</script>
<template>
  <section class="module-page timeline-page" aria-label="时间轴">
    <template v-if="details"
      ><header class="details-heading">
        <button class="icon-button" aria-label="返回时间轴" @click="ui.selectedMatterId = null">
          ‹</button
        ><span class="spacer" /><button
          class="icon-button"
          aria-label="编辑事项"
          @click="editMatter(details.id)"
        >
          <Icon name="edit" /></button
        ><button
          class="icon-button"
          :disabled="!!busy"
          aria-label="删除整个事项"
          @click="remove('remove', { id: details.id })"
        >
          <Icon name="trash" />
        </button>
      </header>
      <div ref="detailsScroll" class="matter-detail-body" @scroll="saveDetailsScroll">
        <div class="matter-title">
          <h2>{{ details.name }}</h2>
          <span class="timeline-status">{{ statusShort(details.status) }}</span>
        </div>
        <div class="matter-stats">
          <div>
            <strong>{{ human(duration(details, now)) }}</strong><span>累计投入</span>
          </div>
          <div>
            <strong>{{ details.sessions.length }}</strong><span>次投入</span>
          </div>
        </div>
        <Resources :owner="{ kind: 'matter', id: details.id }" :resources="details.resources" />
        <div class="session-list">
          <article
            v-for="(session, index) in details.sessions"
            :key="session.id"
            class="session-card"
          >
            <div class="session-card-heading">
              <span>第 {{ index + 1 }} 次</span><strong>{{ human(duration(session, now)) }}</strong
              ><span class="spacer" /><button
                class="icon-button"
                :disabled="!session.endTime"
                :title="session.endTime ? '编辑本次记录' : '先暂停，再修改时间'"
                :aria-label="`编辑第 ${index + 1} 次投入`"
                @click="editSession(details, session)"
              >
                <Icon name="edit" /></button
              ><button
                class="icon-button"
                :disabled="!!busy"
                :aria-label="`删除第 ${index + 1} 次投入`"
                @click="remove('delete-session', { activityId: details.id, sessionId: session.id })"
              >
                <Icon name="trash" />
              </button>
            </div>
            <div class="muted small">
              {{ day(Date.parse(session.startTime)) }} · {{ time(session.startTime) }}—{{
                time(session.endTime)
              }}
            </div>
            <p v-if="session.progress" class="session-progress">{{ session.progress }}</p>
            <div v-if="session.segments.length > 1" class="muted small">
              {{ session.segments.length }} 段有效时间
            </div>
          </article>
        </div>
        <button class="quiet-button add-session" @click="record(details.id)">
          <Icon name="plus" />补录一次投入
        </button>
      </div></template
    ><template v-else
      ><div class="timeline-toolbar">
        <button aria-label="前一天" @click="move(-1)">‹</button
        ><input v-model="ui.timelineDate" type="date" aria-label="时间轴日期" /><button
          aria-label="后一天"
          @click="move(1)"
        >
          ›</button
        ><button @click="ui.timelineDate = day(now)">今天</button>
      </div>
      <div class="timeline-summary">
        <span>{{ human(total) }}</span>
        <div class="timeline-tools">
          <button
            class="icon-button"
            aria-label="颜色说明"
            :aria-expanded="legend"
            @click="legend = !legend"
          >
            <Icon name="info" /></button
          ><button class="icon-button" title="补录" aria-label="补录" @click="record()">
            <Icon name="plus" />
          </button>
        </div>
      </div>
      <div v-if="legend" class="heat-legend">
        <p>颜色区分事项，深浅表示时长</p>
        <span>＜15 分 · 15–30 · 30–60 · ≥60 分</span>
      </div>
      <div ref="timelineScroll" class="timeline-list" @scroll="saveTimelineScroll">
        <button
          v-if="ui.focusedMatterId"
          class="focus-clear quiet-button"
          @click="ui.focusedMatterId = null"
        >
          取消聚焦 ×
        </button>
        <article
          v-for="entry in list"
          :key="entry.session.id"
          class="timeline-entry"
          :style="tint(entry.matter.color || '#2e5a49', entry.duration)"
          @contextmenu.prevent="menu = menu === entry.session.id ? null : entry.session.id"
        >
          <div class="timeline-entry-main">
            <button class="timeline-name" @click="ui.selectedMatterId = entry.matter.id">
              {{ entry.matter.name }}</button><span class="timeline-duration">{{
              human(entry.duration)
            }}</span>
          </div>
          <div class="timeline-entry-meta">
            <span
              >{{ time(new Date(entry.start).toISOString()) }}—{{
                entry.session.endTime ? time(new Date(entry.end).toISOString()) : '进行中'
              }}</span
            ><button
              class="color-focus"
              :style="{ background: entry.matter.color }"
              title="聚焦此事项"
              :aria-label="`聚焦 ${entry.matter.name}`"
              @click="
                ui.focusedMatterId = ui.focusedMatterId === entry.matter.id ? null : entry.matter.id
              "
            />
          </div>
          <p v-if="entry.session.progress" class="timeline-progress">
            {{ entry.session.progress }}
          </p>
          <div v-if="menu === entry.session.id" class="inline-menu">
            <button
              class="icon-button"
              title="详情"
              aria-label="详情"
              @click="
                ui.selectedMatterId = entry.matter.id;
                menu = null;
              "
            >
              <Icon name="arrow" /></button
            ><button
              class="icon-button"
              :title="entry.session.endTime ? '编辑本次' : '先暂停，再修改时间'"
              aria-label="编辑本次"
              :disabled="!entry.session.endTime"
              @click="
                editSession(entry.matter, entry.session);
                menu = null;
              "
            >
              <Icon name="edit" /></button
            ><button
              class="icon-button"
              title="移入回收站"
              aria-label="移入回收站"
              @click="
                remove('delete-session', {
                  activityId: entry.matter.id,
                  sessionId: entry.session.id,
                })
              "
            >
              <Icon name="trash" />
            </button>
          </div>
        </article>
        <button
          v-for="blank in blanks"
          :key="blank.start"
          class="timeline-gap"
          :title="`${time(new Date(blank.start).toISOString())}—${time(new Date(blank.end).toISOString())}`"
          @click="record(undefined, blank.start, blank.end)"
        >
          <span
            >{{ time(new Date(blank.start).toISOString()) }}—{{
              time(new Date(blank.end).toISOString())
            }}</span
          >未记录 · {{ human(blank.end - blank.start) }}
        </button>
        <p v-if="!list.length" class="empty-state">这一天，还没有投入记录。</p>
      </div>
    </template>
  </section>
</template>
