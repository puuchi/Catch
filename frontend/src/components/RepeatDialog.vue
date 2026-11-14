<script setup lang="ts">
import { computed, ref } from 'vue';
import Modal from './Modal.vue';
import { busy, dispatch, state, ui } from '../store';
import type { Schedule } from '../models';
const error = ref('');
const kind = computed({
  get: () => ui.repeatDraft?.schedule.kind || 'daily',
  set: (value) => {
    if (ui.repeatDraft)
      ui.repeatDraft.schedule =
        value === 'weekly'
          ? { kind: value, weekdays: [0, 1, 2, 3, 4] }
          : value === 'every'
            ? { kind: value, days: 2 }
            : value === 'monthly'
              ? { kind: value, day: 1 }
              : { kind: 'daily' };
  },
});
function weekday(value: number) {
  const schedule = ui.repeatDraft?.schedule;
  if (schedule?.kind !== 'weekly') return;
  schedule.weekdays = schedule.weekdays.includes(value)
    ? schedule.weekdays.filter((day) => day !== value)
    : [...schedule.weekdays, value].sort();
}
async function save() {
  const draft = ui.repeatDraft;
  if (!draft) return;
  try {
    if (draft.schedule.kind === 'weekly' && !draft.schedule.weekdays.length)
      throw Error('至少选一个星期');
    if (draft.mode === 'composer') {
      ui.composerSchedule = JSON.parse(JSON.stringify(draft.schedule)) as Schedule;
      ui.composerStartsOn = draft.startsOn;
      ui.repeatDraft = null;
      return;
    }
    if (draft.mode === 'convert')
      await dispatch('recurrence', 'convert', {
        id: draft.todoId,
        schedule: draft.schedule,
        startsOn: draft.startsOn,
      });
    else {
      const template = state.value?.recurrences.templates.find((t) => t.id === draft.templateId);
      if (!template) throw Error('周期规则已不存在');
      await dispatch('recurrence', 'update', {
        id: template.id,
        fields: template.fields,
        resources: template.resources,
        schedule: draft.schedule,
      });
    }
    ui.repeatDraft = null;
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}
async function stop() {
  const draft = ui.repeatDraft;
  if (!draft) return;
  try {
    if (draft.mode === 'composer') ui.composerSchedule = null;
    else if (draft.templateId) await dispatch('recurrence', 'stop', { id: draft.templateId });
    ui.repeatDraft = null;
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}
</script>
<template>
  <Modal v-if="ui.repeatDraft" title="重复" @close="ui.repeatDraft = null"
    ><form novalidate @submit.prevent="save">
      <div class="editor-body">
        <select v-model="kind" aria-label="重复方式">
          <option value="daily">每天</option>
          <option value="weekly">每周</option>
          <option value="every">间隔天数</option>
          <option value="monthly">每月</option>
        </select>
        <div
          v-if="ui.repeatDraft.schedule.kind === 'weekly'"
          class="weekday-picker"
          aria-label="重复星期"
        >
          <button
            v-for="(label, index) in ['一', '二', '三', '四', '五', '六', '日']"
            :key="index"
            type="button"
            :class="{ selected: ui.repeatDraft.schedule.weekdays.includes(index) }"
            :aria-pressed="ui.repeatDraft.schedule.weekdays.includes(index)"
            @click="weekday(index)"
          >
            {{ label }}
          </button>
        </div>
        <label v-if="ui.repeatDraft.schedule.kind === 'every'" class="inline-field"
          >每
          <input
            v-model.number="ui.repeatDraft.schedule.days"
            type="number"
            min="1"
            max="3650"
            aria-label="间隔天数"
          />
          天</label
        ><template v-if="ui.repeatDraft.schedule.kind === 'monthly'"
          ><label class="inline-field"
            >每月
            <input
              v-model.number="ui.repeatDraft.schedule.day"
              type="number"
              min="1"
              max="31"
              aria-label="每月日期"
            />
            日</label
          >
          <p class="muted small">不足该日期时，取当月最后一天</p></template
        ><label v-if="ui.repeatDraft.mode !== 'template'" class="stack-field"
          >从这一天开始<input
            v-model="ui.repeatDraft.startsOn"
            type="date"
            aria-label="周期开始日期"
        /></label>
        <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
        <button
          v-if="ui.repeatDraft.mode !== 'convert'"
          type="button"
          class="quiet-button"
          @click="stop"
        >
          {{ ui.repeatDraft.mode === 'composer' ? '不重复' : '停止重复' }}
        </button>
      </div>
      <footer class="editor-buttons">
        <button type="button" @click="ui.repeatDraft = null">取消</button
        ><button class="primary" :disabled="!!busy">确定</button>
      </footer>
    </form></Modal
  >
</template>
