<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import Modal from './Modal.vue';
import DateField from './DateField.vue';
import TimeField from './TimeField.vue';
import { busy, dispatch, now, state } from '../store';
import { day, effective, human, inputTime, time } from '../display';
import type { Entry } from '../display';
import { adjacentRows } from '../timeline';
const props = defineProps<{ kind: 'merge' | 'split'; entry: Entry; date: string }>();
const emit = defineEmits<{
  close: [];
  complete: [matterId: string, sessionId: string, date: string];
}>();
const openedAt = now.value;
const pending = ref(false);
const error = ref('');
const locked = computed(() => pending.value || !!busy.value);
const candidates = computed(() =>
  adjacentRows(
    state.value?.tracking.activities || [],
    props.date,
    openedAt,
    props.entry.session.id,
  ),
);
const selectedKey = ref(candidates.value[0]?.key || '');
const selected = computed(() => candidates.value.find((row) => row.key === selectedKey.value));
const targetId = ref(props.entry.matter.id);
watch(selectedKey, () => {
  targetId.value = props.entry.matter.id;
  error.value = '';
});
const targetOptions = computed(() => {
  const values = [props.entry.matter];
  if (selected.value?.kind === 'entry' && selected.value.entry.matter.id !== props.entry.matter.id)
    values.push(selected.value.entry.matter);
  return values;
});
const point = ref(
  inputTime(new Date(Math.floor((props.entry.start + props.entry.end) / 2)).toISOString()),
);
const pointDate = computed({
  get: () => point.value.slice(0, 10),
  set: (date: string) => {
    point.value = `${date}${point.value.slice(10)}`;
  },
});
const splitDurations = computed(() => {
  const at = new Date(point.value).getTime();
  return [
    effective(props.entry.session.segments, openedAt, [-Infinity, at]),
    effective(props.entry.session.segments, openedAt, [at, Infinity]),
  ];
});
async function save() {
  if (locked.value) return;
  error.value = '';
  pending.value = true;
  try {
    if (props.kind === 'split') {
      const at = new Date(point.value).getTime();
      const from = Date.parse(props.entry.session.startTime);
      const to = props.entry.session.endTime ? Date.parse(props.entry.session.endTime) : openedAt;
      if (
        !Number.isFinite(at) ||
        at <= from ||
        at >= to ||
        splitDurations.value.some((duration) => duration <= 0)
      )
        throw Error('请选择投入内部的拆分时间，两侧都需要有有效投入');
      await dispatch('tracking', 'split-session', {
        activityId: props.entry.matter.id,
        sessionId: props.entry.session.id,
        atTime: new Date(at).toISOString(),
      });
      emit('complete', props.entry.matter.id, props.entry.session.id, props.date);
    } else {
      const neighbor = selected.value;
      if (!neighbor) throw Error('相邻记录已变化，请关闭后重新选择');
      await dispatch('tracking', 'merge-sessions', {
        first: { activityId: props.entry.matter.id, sessionId: props.entry.session.id },
        targetActivityId: targetId.value,
        ...(neighbor.kind === 'entry'
          ? {
              second: {
                activityId: neighbor.entry.matter.id,
                sessionId: neighbor.entry.session.id,
              },
            }
          : {
              gap: {
                startTime: new Date(neighbor.start).toISOString(),
                endTime: new Date(neighbor.end).toISOString(),
              },
            }),
      });
      const retained =
        neighbor.kind === 'entry' &&
        neighbor.entry.matter.id === targetId.value &&
        props.entry.matter.id !== targetId.value
          ? neighbor.entry.session.id
          : props.entry.session.id;
      emit('complete', targetId.value, retained, props.date);
    }
  } catch (cause) {
    error.value = (cause as Error).message;
  } finally {
    pending.value = false;
  }
}
</script>
<template>
  <Modal
    :title="kind === 'merge' ? '合并相邻段' : '拆分本次投入'"
    @close="!locked && emit('close')"
  >
    <form novalidate @submit.prevent="save">
      <div class="editor-body transform-body">
        <p class="transform-name">{{ entry.matter.name }}</p>
        <template v-if="kind === 'merge'">
          <label class="transform-label"
            >合并相邻段<select v-model="selectedKey" aria-label="合并相邻段" :disabled="locked">
              <option v-if="!candidates.length" value="">没有可合并的相邻段</option>
              <option v-for="row in candidates" :key="row.key" :value="row.key">
                {{ time(new Date(row.start).toISOString()) }}—{{
                  time(new Date(row.end).toISOString())
                }}
                · {{ row.kind === 'entry' ? row.entry.matter.name : '未计时' }}
              </option>
            </select></label
          >
          <label v-if="targetOptions.length > 1" class="transform-label"
            >合并后归属<select v-model="targetId" aria-label="合并后归属" :disabled="locked">
              <option v-for="matter in targetOptions" :key="matter.id" :value="matter.id">
                {{ matter.name }}
              </option>
            </select></label
          >
          <p class="muted small">
            只合并选中的相邻段，其他历史不动。两条进展按时间顺序保留；未计时段合并后计入本次投入。
          </p>
        </template>
        <template v-else>
          <DateField v-model="pointDate" label="拆分日期" />
          <TimeField v-model="point" label="拆分时间" />
          <p class="muted small">拆成同一事项的两次投入，原进展保留在前半，后半留空。</p>
        </template>
        <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
      </div>
      <footer class="editor-buttons transform-buttons">
        <output v-if="kind === 'split'"
          >{{ human(splitDurations[0]) }} ＋ {{ human(splitDurations[1]) }}</output
        >
        <button type="button" :disabled="locked" @click="emit('close')">取消</button>
        <button class="primary" :disabled="locked || (kind === 'merge' && !selected)">
          {{ kind === 'merge' ? '合并' : '拆分' }}
        </button>
      </footer>
    </form>
  </Modal>
</template>
<style scoped>
.transform-name {
  font-weight: 700;
  overflow-wrap: anywhere;
}
.transform-label {
  display: flex;
  flex-direction: column;
  gap: 5px;
  font-size: 10px;
  color: var(--text-muted);
  min-width: 0;
}
.transform-label select {
  width: 100%;
  min-width: 0;
  padding: 7px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--panel);
  font-size: 11px;
}
.transform-buttons {
  flex-wrap: wrap;
}
.transform-buttons output {
  flex: 1;
  min-width: 65px;
  font-size: 9px;
  color: var(--text-muted);
  overflow-wrap: anywhere;
}
</style>
