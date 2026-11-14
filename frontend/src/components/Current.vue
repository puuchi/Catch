<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import Icon from './Icon.vue';
import Resources from './Resources.vue';
import ValidationHint from './ValidationHint.vue';
import { busy, dispatch, inform, now, state, ui } from '../store';
import { clock, day, duration, foreground, human, statusText, time } from '../display';
import { useScroll } from '../scroll';
const scroll = ref<HTMLElement>();
const saveScroll = useScroll(scroll, () => 'current');
const current = computed(() => foreground(state.value?.tracking.activities || [], ui.foregroundId));
const background = computed(
  () =>
    state.value?.tracking.activities.filter(
      (m) => m.status === 'paused' && m.id !== current.value?.id,
    ) || [],
);
const lastSession = computed(() => current.value?.sessions.at(-1));
const validation = ref('');
watch(
  current,
  (value) => {
    if (value) ui.foregroundId = value.id;
  },
  { immediate: true },
);
const progress = computed({
  get: () =>
    lastSession.value
      ? (ui.progressDrafts[lastSession.value.id] ?? lastSession.value.progress)
      : '',
  set: (value) => {
    if (lastSession.value) ui.progressDrafts[lastSession.value.id] = value;
  },
});
async function action(type: string, payload: Record<string, unknown> = {}) {
  try {
    if (lastSession.value && progress.value !== lastSession.value.progress) await saveProgress();
    await dispatch('tracking', type, payload);
  } catch (error) {
    inform(error);
  }
}
function blurProgress(event: FocusEvent) {
  if ((event.relatedTarget as HTMLElement | null)?.closest('.activity-actions,.progress-form'))
    return;
  if (lastSession.value && progress.value !== lastSession.value.progress) void saveProgress();
}
async function start() {
  if (!ui.activityName.trim()) {
    validation.value = '先写下准备做的事';
    return;
  }
  try {
    await dispatch('tracking', 'start', { name: ui.activityName });
    ui.activityName = '';
  } catch (error) {
    validation.value = (error as Error).message;
  }
}
async function saveProgress(next = false) {
  const matter = current.value,
    session = lastSession.value;
  if (!matter || !session) return;
  try {
    await dispatch('tracking', next ? 'next-session' : 'set-progress', {
      activityId: matter.id,
      sessionId: session.id,
      progress: progress.value,
    });
    delete ui.progressDrafts[session.id];
  } catch (error) {
    inform(error);
  }
}
function today() {
  ui.timelineDate = day(now.value);
  ui.selectedMatterId = null;
  ui.page = 'timeline';
}
</script>
<template>
  <section ref="scroll" class="module-page activity-page" aria-label="正在做" @scroll="saveScroll">
    <div class="current-card" :data-resource-kind="'matter'" :data-resource-id="current?.id">
      <div class="card-eyebrow">
        <span class="current-status-label"
          ><i class="status-dot" :class="current?.status" /><span>{{
            statusText(current?.status)
          }}</span></span
        ><time v-if="current" class="muted small">{{ time(current.startTime) }} 开始</time>
      </div>
      <h1 id="currentName">{{ current?.name || '未开始活动' }}</h1>
      <div class="current-timer-row">
        <div class="current-timer">{{ clock(current ? duration(current, now) : 0) }}</div>
        <div v-if="current && current.status !== 'completed'" class="activity-actions">
          <button
            :disabled="!!busy"
            :title="current.status === 'running' ? '暂停' : '继续'"
            :aria-label="current.status === 'running' ? '暂停当前事项' : '继续当前事项'"
            @click="
              action(
                current.status === 'running' ? 'pause' : 'resume',
                current.status === 'running' ? {} : { id: current.id },
              )
            "
          >
            <Icon :name="current.status === 'running' ? 'pause' : 'play'" /></button
          ><button
            title="结束"
            aria-label="结束整个事项"
            :disabled="!!busy"
            @click="action('end', { id: current.id })"
          >
            <Icon name="stop" />
          </button>
        </div>
      </div>
      <div v-if="current?.status === 'completed'" class="completed-summary">
        <span>{{ current.sessions.length }} 次投入</span
        ><button
          :disabled="!!busy"
          @click="
            action('update-matter', { id: current.id, name: current.name, status: 'running' })
          "
        >
          <Icon name="play" />重新开始
        </button>
      </div>
      <form
        v-if="current && lastSession && current.status !== 'completed'"
        class="progress-form"
        @submit.prevent="saveProgress()"
      >
        <input
          v-model="progress"
          aria-label="本次进展"
          placeholder="本次进展…"
          maxlength="2000"
          @blur="blurProgress"
        /><button
          type="button"
          :disabled="!!busy || current.status !== 'running' || !progress.trim()"
          title="保存本次并开始下一步"
          aria-label="保存本次并开始下一步"
          @pointerdown.prevent
          @click="saveProgress(true)"
        >
          <Icon name="next" />
        </button>
      </form>
      <Resources
        v-if="current"
        :owner="{ kind: 'matter', id: current.id }"
        :resources="current.resources"
      />
    </div>
    <section
      v-if="background.length"
      class="background-activities"
      :class="{ collapsed: !ui.backgroundOpen }"
    >
      <div class="background-toggle">
        <span>后台 · {{ background.length }}</span
        ><button
          class="icon-button"
          :aria-expanded="ui.backgroundOpen"
          aria-label="展开或收起后台事项"
          @click="ui.backgroundOpen = !ui.backgroundOpen"
        >
          <Icon name="chevron" />
        </button>
      </div>
      <div v-if="ui.backgroundOpen" class="paused-activity-list">
        <div v-for="matter in background" :key="matter.id" class="background-item">
          <div class="background-content">
            <div class="background-name">{{ matter.name }}</div>
            <span class="muted small">{{ human(duration(matter, now)) }}</span>
          </div>
          <button
            class="icon-button"
            :disabled="!!busy"
            :aria-label="`继续 ${matter.name}`"
            title="继续"
            @click="action('resume', { id: matter.id })"
          >
            <Icon name="play" /></button
          ><button
            class="icon-button"
            :disabled="!!busy"
            :aria-label="`结束 ${matter.name}`"
            title="结束"
            @click="action('end', { id: matter.id })"
          >
            <Icon name="stop" />
          </button>
        </div>
      </div>
    </section>
    <form class="start-activity-form" novalidate @submit.prevent="start">
      <div class="start-activity-heading">
        <label for="activityName">开始新活动</label
        ><button class="quiet-button" type="button" @click="today">今日时间轴 →</button>
      </div>
      <div class="activity-input-row field-wrap">
        <input
          id="activityName"
          v-model="ui.activityName"
          placeholder="你现在准备做什么？"
          maxlength="200"
          autocomplete="off"
          @input="validation = ''"
        /><button class="primary" :disabled="!!busy">开始</button
        ><ValidationHint :message="validation" @dismiss="validation = ''" />
      </div>
    </form>
  </section>
</template>
