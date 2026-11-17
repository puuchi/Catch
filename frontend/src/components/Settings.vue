<script setup lang="ts">
import { computed, ref } from 'vue';
import PlantSprite from './PlantSprite.vue';
import Icon from './Icon.vue';
import LegacyImport from './LegacyImport.vue';
import { busy, dispatch, growth, preferences, savePreferences, state, ui } from '../store';
import type { Plant } from '../models';
withDefaults(defineProps<{ mode?: 'appearance' | 'growth' }>(), { mode: 'appearance' });
const plants: { id: Plant; name: string }[] = [
  { id: 'tulip', name: '郁金香' },
  { id: 'sunflower', name: '向日葵' },
  { id: 'corn', name: '玉米' },
  { id: 'strawberry', name: '草莓' },
  { id: 'sprout', name: '绿芽' },
];
const preview = computed(() => ui.growthPreview || growth.value?.plant || 'sprout');
const error = ref('');
const browsing = ref(false);
const catalogPlant = ref<Plant>('sprout');
function browsePlants() {
  if (!browsing.value) catalogPlant.value = growth.value?.plant || 'sprout';
  browsing.value = !browsing.value;
}
const stages = ['初始', '萌芽', '生长', '茂盛', '成熟'];
const nextStageMinutes = computed(() => {
  const current = growth.value;
  if (!current || current.stage >= 4) return 0;
  const threshold = [0.1, 0.3, 0.6, 1][current.stage];
  return Math.max(
    1,
    Math.ceil(((current.goalMinutes || 60) * 60_000 * threshold - current.investedMs) / 60_000),
  );
});
const goal = computed({
  get: () => ui.goalDraft ?? String(state.value?.growth.goalMinutes || 60),
  set: (value) => {
    ui.goalDraft = value;
  },
});
async function choose() {
  try {
    await dispatch('growth', 'choose', { plant: preview.value });
    ui.growthPreview = null;
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}
async function setGoal(enabled = true) {
  try {
    const minutes = Number(goal.value);
    if (enabled && (!Number.isInteger(minutes) || minutes < 1 || minutes > 1440))
      throw Error('目标须为 1–1440 分钟');
    await dispatch('growth', 'set-goal', { minutes: enabled ? minutes : null });
    ui.goalDraft = null;
    error.value = '';
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}
</script>
<template>
  <section
    class="settings-panel"
    :class="{ 'growth-panel': mode === 'growth' }"
    :aria-label="mode === 'growth' ? '今日植物成长' : '外观设置'"
  >
    <div v-if="mode === 'appearance'" class="settings-section">
      <label class="settings-row" for="backgroundTransparency"
        >背景透明度 <output>{{ preferences.transparency }}%</output></label
      ><input
        id="backgroundTransparency"
        v-model.number="preferences.transparency"
        type="range"
        min="0"
        max="100"
        aria-label="背景透明度"
        @change="savePreferences"
      />
    </div>
    <template v-if="mode === 'growth'">
      <div class="settings-row growth-panel-heading">
        <strong>{{ browsing ? '查看植物' : '今日成长' }}</strong>
        <button
          type="button"
          class="icon-button"
          :title="browsing ? '返回今日成长' : '查看植物'"
          :aria-label="browsing ? '返回今日成长' : '查看植物'"
          :aria-pressed="browsing"
          @click="browsePlants"
        >
          <Icon :name="browsing ? 'back' : 'plants'" />
        </button>
      </div>
      <div v-if="browsing" class="settings-section" aria-label="植物浏览">
        <div class="plant-preview">
          <PlantSprite :plant="catalogPlant" :stage="4" :size="56" /><strong>{{
            plants.find((plant) => plant.id === catalogPlant)?.name
          }}</strong>
        </div>
        <div class="plant-choices">
          <button
            v-for="plant in plants"
            :key="plant.id"
            :class="{ selected: catalogPlant === plant.id }"
            :aria-pressed="catalogPlant === plant.id"
            :aria-label="`查看${plant.name}`"
            @click="catalogPlant = plant.id"
          >
            <PlantSprite :plant="plant.id" :stage="4" :size="32" /><span>{{ plant.name }}</span>
          </button>
        </div>
        <div
          class="growth-stages"
          :aria-label="`${plants.find((plant) => plant.id === catalogPlant)?.name}的阶段形态`"
        >
          <div v-for="(label, index) in stages" :key="label" class="growth-stage">
            <PlantSprite :plant="catalogPlant" :stage="index" :size="32" /><span>{{ label }}</span>
          </div>
        </div>
      </div>
      <div v-else-if="!growth?.selected" class="settings-section">
        <div class="plant-choices">
          <button
            v-for="plant in plants"
            :key="plant.id"
            :class="{ selected: preview === plant.id }"
            :aria-pressed="preview === plant.id"
            @click="ui.growthPreview = plant.id"
          >
            <PlantSprite :plant="plant.id" :stage="4" :size="32" /><span>{{ plant.name }}</span>
          </button>
        </div>
        <div class="plant-preview">
          <PlantSprite :plant="preview" :stage="growth?.stage || 0" :size="56" />
          <div>
            <span class="muted small">{{
              growth?.selected && preview === growth.plant ? '今天的植物' : '预览'
            }}</span><strong>{{ plants.find((p) => p.id === preview)?.name }}</strong>
          </div>
          <button class="primary" :disabled="!!busy || growth?.selected" @click="choose">
            {{ growth?.selected ? '今天已选定' : '今天选它' }}
          </button>
        </div>
        <p v-if="growth?.selected" class="muted small">明天可以换一种。</p>
      </div>
      <div v-else-if="growth" class="settings-section">
        <div class="plant-preview">
          <PlantSprite :plant="growth.plant" :stage="growth.stage" :size="48" />
          <div>
            <span class="muted small"
              >今天的{{ plants.find((plant) => plant.id === growth?.plant)?.name }}</span
            ><strong>{{ stages[growth.stage] }}{{ growth.stage === 4 ? '了' : '中' }}</strong>
          </div>
        </div>
        <div class="growth-stages" aria-label="五个成长阶段">
          <div
            v-for="(label, index) in stages"
            :key="label"
            class="growth-stage"
            :class="{ future: index > growth.stage }"
            :aria-current="index === growth.stage ? 'step' : undefined"
            :aria-label="`${label}${index === growth.stage ? '，当前阶段' : index < growth.stage ? '，已达到' : '，尚未达到'}`"
          >
            <PlantSprite :plant="growth.plant" :stage="index" :size="32" />
            <span>{{ label }}</span><span class="growth-stage-marker"
              ><template v-if="index === growth.stage">当前</template
              ><Icon v-else-if="index < growth.stage" name="check"
            /></span>
          </div>
        </div>
        <p class="growth-next-stage">
          {{
            growth.stage === 4
              ? '今天的植物已经长成'
              : `再投入 ${nextStageMinutes} 分钟，长到${stages[growth.stage + 1]}阶段`
          }}
        </p>
      </div>
      <div class="settings-section">
        <div class="settings-row">
          <label for="growthGoal">每日目标</label
          ><label class="toggle-control"
            ><input
              type="checkbox"
              aria-label="开启每日目标"
              :disabled="!!busy"
              :checked="state?.growth.goalMinutes !== null"
              @change="setGoal(state?.growth.goalMinutes === null)"
          /></label>
        </div>
        <form
          v-if="state?.growth.goalMinutes !== null"
          class="goal-input"
          @submit.prevent="setGoal()"
        >
          <input
            id="growthGoal"
            v-model="goal"
            type="number"
            min="1"
            max="1440"
            aria-label="每日目标分钟数"
          /><span>分钟</span
          ><button type="submit" class="quiet-button" :disabled="!!busy">保存</button>
        </form>
      </div>
    </template>
    <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
    <LegacyImport v-if="mode === 'appearance'" />
  </section>
</template>
