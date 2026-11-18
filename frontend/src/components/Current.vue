<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import Icon from './Icon.vue';
import FitTimer from './FitTimer.vue';
import Modal from './Modal.vue';
import Resources from './Resources.vue';
import ValidationHint from './ValidationHint.vue';
import { busy, dispatch, flushUi, inform, now, state, ui } from '../store';
import {
  clock,
  day,
  duration,
  foreground,
  human,
  pausedMatters,
  statusText,
  time,
} from '../display';
import type { Matter, Session } from '../models';
import { useScroll } from '../scroll';
const scroll = ref<HTMLElement>();
const saveScroll = useScroll(scroll, () => 'current');
const current = computed(() =>
  foreground(state.value?.tracking.activities || [], ui.foregroundId, ui.foregroundClosed),
);
const background = computed(() =>
  pausedMatters(state.value?.tracking.activities || [], current.value?.id),
);
const lastSession = computed(() => current.value?.sessions.at(-1));
const lastProgress = computed(
  () =>
    current.value?.sessions
      .slice()
      .reverse()
      .find((session) => session.progress.trim())?.progress,
);
const validation = ref('');
const pending = ref(false);
const locked = computed(() => pending.value || !!busy.value);
const editingProgress = ref(false);
const progressInput = ref<HTMLInputElement>();
const endingId = ref<string | null>(null);
const finishError = ref('');
const ending = computed(() =>
  state.value?.tracking.activities.find((matter) => matter.id === endingId.value),
);
function linkedTodo(matter: Matter) {
  return state.value?.todos.todos.find(
    (todo) => todo.id === matter.todoId && !todo.done && !todo.deletedAt,
  );
}
watch(
  current,
  (value) => {
    if (value) {
      ui.foregroundId = value.id;
      ui.foregroundClosed = false;
    }
  },
  { immediate: true },
);
watch(
  () => lastSession.value?.id,
  () => {
    const session = lastSession.value;
    editingProgress.value = !!session && (session.id in ui.progressDrafts || !session.progress);
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
async function run(operation: () => Promise<void>) {
  if (locked.value) return false;
  pending.value = true;
  try {
    await operation();
    return true;
  } catch (error) {
    inform(error);
    return false;
  } finally {
    pending.value = false;
  }
}
function clearDraft(session: Session, saved: string) {
  if (ui.progressDrafts[session.id] === saved) delete ui.progressDrafts[session.id];
}
async function persistProgress() {
  const matter = current.value;
  const session = lastSession.value;
  if (!matter || !session || matter.status === 'completed') return;
  const saved = progress.value;
  if (saved !== session.progress) {
    await dispatch('tracking', 'set-progress', {
      activityId: matter.id,
      sessionId: session.id,
      progress: saved,
    });
  }
  clearDraft(session, saved);
  editingProgress.value = false;
}
async function action(type: string, payload: Record<string, unknown> = {}) {
  await run(async () => {
    await persistProgress();
    await dispatch('tracking', type, payload);
  });
}
function blurProgress(event: FocusEvent) {
  // These actions explicitly save/finish the draft themselves, so blur must not race them.
  if (
    (event.relatedTarget as HTMLElement | null)?.closest(
      '.activity-actions,.progress-form,.background-item,.start-activity-form',
    )
  )
    return;
  void saveProgress();
}
async function editProgress() {
  editingProgress.value = true;
  await nextTick();
  progressInput.value?.focus();
}
async function start() {
  if (!ui.activityName.trim()) {
    validation.value = '先写下准备做的事';
    return;
  }
  await run(async () => {
    await persistProgress();
    await dispatch('tracking', 'start', { name: ui.activityName });
    ui.activityName = '';
  });
}
async function saveProgress(next = false, clickCount = 0) {
  if (next && clickCount > 1) return;
  await run(async () => {
    const matter = current.value;
    const session = lastSession.value;
    if (!matter || !session || matter.status === 'completed') return;
    if (!next) {
      await persistProgress();
      return;
    }
    const saved = progress.value;
    await dispatch('tracking', 'next-session', {
      activityId: matter.id,
      sessionId: session.id,
      progress: saved,
    });
    clearDraft(session, saved);
  });
}
function requestEnd(matter: Matter) {
  if (locked.value) return;
  finishError.value = '';
  if (linkedTodo(matter)) endingId.value = matter.id;
  else void finish(matter, false);
}
async function finish(matter: Matter, completeTodo: boolean) {
  const success = await run(async () => {
    // Finish carries the target's last draft in the same reliable operation as end/todo completion.
    if (matter.id !== current.value?.id) await persistProgress();
    const session = matter.sessions.at(-1);
    const saved = session ? ui.progressDrafts[session.id] : undefined;
    try {
      await dispatch('finish', 'finish', {
        activityId: matter.id,
        completeTodo,
        ...(session && saved !== undefined ? { sessionId: session.id, progress: saved } : {}),
      });
    } catch (error) {
      finishError.value = (error as Error).message;
      throw error;
    }
    if (session && saved !== undefined) clearDraft(session, saved);
    editingProgress.value = false;
    endingId.value = null;
  });
  return success;
}
async function closeCompleted() {
  if (locked.value || current.value?.status !== 'completed') return;
  const id = ui.foregroundId;
  ui.foregroundClosed = true;
  ui.foregroundId = null;
  try {
    await flushUi();
  } catch (error) {
    ui.foregroundId = id;
    ui.foregroundClosed = false;
    inform(error);
  }
}
function details() {
  if (!current.value) return;
  ui.selectedMatterId = current.value.id;
  ui.selectedSessionId = null;
  ui.selectedSessionDate = null;
  ui.page = 'timeline';
}
function today() {
  ui.timelineDate = day(now.value);
  ui.selectedMatterId = null;
  ui.page = 'timeline';
}
</script>
<template>
  <section class="module-page activity-page" aria-label="正在做">
    <div ref="scroll" class="current-content" @scroll="saveScroll">
      <div
        v-if="current"
        class="current-card"
        data-resource-kind="matter"
        :data-resource-id="current.id"
      >
        <div class="card-eyebrow">
          <span class="current-status-label"
            ><i class="status-dot" :class="current.status" /><span>{{
              statusText(current.status)
            }}</span></span
          >
          <time class="muted small">{{ time(current.startTime) }} 开始</time>
          <button
            v-if="current.status === 'completed'"
            class="icon-button close-completed"
            v-tooltip="'关闭卡片，保留记录'"
            aria-label="关闭已结束卡片"
            :disabled="locked"
            @click="closeCompleted"
          >
            <Icon name="close" />
          </button>
        </div>
        <h1 id="currentName">{{ current.name }}</h1>
        <div class="current-timer-row">
          <FitTimer :text="clock(duration(current, now))" />
          <div v-if="current.status !== 'completed'" class="activity-actions">
            <button
              :disabled="locked"
              v-tooltip="current.status === 'running' ? '暂停' : '继续'"
              :aria-label="current.status === 'running' ? '暂停当前事项' : '继续当前事项'"
              @click="
                action(
                  current.status === 'running' ? 'pause' : 'resume',
                  current.status === 'running' ? {} : { id: current.id },
                )
              "
            >
              <Icon :name="current.status === 'running' ? 'pause' : 'play'" />
            </button>
            <button
              v-tooltip="'结束'"
              aria-label="结束整个事项"
              :disabled="locked"
              @click="requestEnd(current)"
            >
              <Icon name="stop" />
            </button>
          </div>
        </div>
        <template v-if="current.status === 'completed'">
          <div class="completed-summary">
            <span>{{ current.sessions.length }} 次投入</span>
            <button @click="details"><Icon name="timeline" />查看过程</button>
          </div>
          <p v-if="lastProgress" class="completed-progress">
            <span class="muted small">最后进展</span>{{ lastProgress }}
          </p>
        </template>
        <form v-else-if="lastSession" class="progress-form" @submit.prevent="saveProgress()">
          <input
            v-if="editingProgress"
            ref="progressInput"
            v-model="progress"
            aria-label="本次进展"
            placeholder="本次进展…"
            maxlength="2000"
            :disabled="locked"
            @blur="blurProgress"
          />
          <template v-else>
            <p class="progress-text">{{ lastSession.progress || '尚未填写进展' }}</p>
            <button
              type="button"
              :disabled="locked"
              v-tooltip="'修改本次进展'"
              aria-label="修改本次进展"
              @click="editProgress"
            >
              <Icon name="edit" />
            </button>
          </template>
          <button
            type="button"
            class="next-session-button"
            :disabled="locked || current.status !== 'running'"
            v-tooltip="'保存本次并开始下一步'"
            aria-label="保存本次并开始下一步"
            @pointerdown.prevent
            @click="saveProgress(true, $event.detail)"
          >
            <Icon name="next" />
          </button>
        </form>
        <Resources :owner="{ kind: 'matter', id: current.id }" :resources="current.resources" />
      </div>
      <p v-else class="empty-state current-empty">从一件小事开始，或继续下方暂停的事项。</p>
      <section
        v-if="background.length"
        class="background-activities"
        :class="{ collapsed: !ui.backgroundOpen }"
      >
        <div class="background-toggle">
          <span>后台 {{ background.length }}</span>
          <button
            class="icon-button"
            :aria-expanded="ui.backgroundOpen"
            v-tooltip="'展开或收起后台事项'"
            aria-label="展开或收起后台事项"
            @click="ui.backgroundOpen = !ui.backgroundOpen"
          >
            <Icon name="chevron" />
          </button>
        </div>
        <div v-if="ui.backgroundOpen" class="paused-activity-list">
          <div v-for="matter in background" :key="matter.id" class="background-item">
            <i
              class="status-dot matter-color"
              :style="{ backgroundColor: matter.color }"
              v-tooltip="'已暂停'"
              aria-label="已暂停"
            />
            <div class="background-content">
              <div class="background-name" v-tooltip="matter.name">{{ matter.name }}</div>
              <span class="muted small">{{ human(duration(matter, now)) }}</span>
            </div>
            <button
              class="icon-button"
              :disabled="locked"
              :aria-label="`继续 ${matter.name}`"
              v-tooltip="'继续'"
              @click="action('resume', { id: matter.id })"
            >
              <Icon name="play" />
            </button>
            <button
              class="icon-button"
              :disabled="locked"
              :aria-label="`结束 ${matter.name}`"
              v-tooltip="'结束'"
              @click="requestEnd(matter)"
            >
              <Icon name="stop" />
            </button>
          </div>
        </div>
      </section>
    </div>
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
        />
        <button class="primary" :disabled="locked">开始</button
        ><ValidationHint :message="validation" @dismiss="validation = ''" />
      </div>
    </form>
    <Modal v-if="ending" title="结束事项" @close="!locked && (endingId = null)">
      <div class="editor-body finish-body">
        <p>是否同时完成关联待办？</p>
        <p class="finish-todo-name">{{ linkedTodo(ending)?.text || ending.name }}</p>
        <p v-if="finishError" class="editor-error" role="alert">{{ finishError }}</p>
      </div>
      <footer class="editor-buttons finish-buttons">
        <button :disabled="locked" autofocus @click="finish(ending, false)">仅结束</button>
        <button class="primary" :disabled="locked" @click="finish(ending, true)">
          结束并完成待办
        </button>
      </footer>
    </Modal>
  </section>
</template>
