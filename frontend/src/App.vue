<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { host } from './bridge';
import {
  busy,
  celebrating,
  collapse,
  dispatch,
  flushUi,
  generation,
  growth,
  inform,
  initialize,
  isolated,
  message,
  now,
  preferences,
  savePreferences,
  state,
  ui,
} from './store';
import Icon from './components/Icon.vue';
import Todos from './components/Todos.vue';
import Current from './components/Current.vue';
import Timeline from './components/Timeline.vue';
import Trash from './components/Trash.vue';
import Editors from './components/Editors.vue';
import ResourceDialog from './components/ResourceDialog.vue';
import RepeatDialog from './components/RepeatDialog.vue';
import Settings from './components/Settings.vue';
import PlantSprite from './components/PlantSprite.vue';
import Modal from './components/Modal.vue';
import { human } from './display';
import type { Owner, Page } from './models';
const ready = ref(false),
  quitConfirm = ref(false);
const growthTop = ref(0);
function positionGrowth() {
  const strip = document.querySelector('.growth-strip');
  if (strip) growthTop.value = strip.getBoundingClientRect().bottom + 4;
}
function toggleGrowth() {
  positionGrowth();
  ui.growthOpen = !ui.growthOpen;
  ui.settingsOpen = false;
}
function toggleAppearance() {
  ui.settingsOpen = !ui.settingsOpen;
  ui.growthOpen = false;
}
const tabs: { id: Page; name: string; icon: string }[] = [
  { id: 'todos', name: '待办', icon: 'list' },
  { id: 'current', name: '正在做', icon: 'timer' },
  { id: 'timeline', name: '时间轴', icon: 'timeline' },
];
const current = computed(() =>
  state.value?.tracking.activities.find((m) => m.status === 'running'),
);
async function reportLayout() {
  if (!ready.value) return;
  await nextTick();
  const rect = (selector: string) => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const r = element.getBoundingClientRect();
    return {
      x: r.x,
      y: r.y,
      width: r.width,
      height: r.height,
      scrollHeight: element.scrollHeight,
      clientHeight: element.clientHeight,
    };
  };
  await host
    .layout(generation.value, {
      width: window.innerWidth,
      height: window.innerHeight,
      dpr: window.devicePixelRatio,
      page: ui.page,
      bodyFont: getComputedStyle(document.body).fontSize,
      app: rect('#app'),
      content: rect('.app-content'),
      pageBounds: rect('.module-page'),
      currentCard: rect('.current-card'),
      startForm: rect('.start-activity-form'),
      footer: rect('.app-tools'),
    })
    .catch(() => {});
}
watch(
  () => [ui.page, ui.selectedMatterId, ui.settingsOpen, ui.growthOpen],
  () => void reportLayout(),
);
window.addEventListener('resize', () => {
  positionGrowth();
  void reportLayout();
});
watch(
  preferences,
  (value) => {
    document.documentElement.style.setProperty(
      '--bg',
      `rgba(249,248,241,${1 - value.transparency / 100})`,
    );
    document.documentElement.style.setProperty(
      '--wash-opacity',
      String((1 - value.transparency / 100) * 0.24),
    );
  },
  { deep: true },
);
function page(value: Page) {
  ui.page = value;
  ui.settingsOpen = false;
  ui.growthOpen = false;
  if (value !== 'trash') ui.previousPage = value;
}
function toggleArchive() {
  const returning = ui.page === 'todos' && ui.history;
  page('todos');
  ui.history = !returning;
}
async function pin() {
  preferences.pinned = !preferences.pinned;
  await savePreferences();
}
async function quit() {
  try {
    await flushUi();
    if (current.value) {
      quitConfirm.value = true;
      return;
    }
    await host.quit(generation.value);
  } catch (error) {
    inform(error);
  }
}
async function confirmedQuit() {
  try {
    await flushUi();
    await host.quit(generation.value);
  } catch (error) {
    inform(error);
  }
}
onMounted(async () => {
  try {
    await initialize();
    ready.value = true;
    await nextTick();
    positionGrowth();
    await document.fonts.ready;
    await Promise.all(Array.from(document.images, (image) => image.decode().catch(() => {})));
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
    await host.painted(generation.value);
    await reportLayout();
    await host.on('catch:request-quit', () => void quit());
    await host.on('catch:request-collapse', () => void collapse());
    await getCurrentWebview().onDragDropEvent(async (event) => {
      if (event.payload.type !== 'drop') return;
      const point = event.payload.position,
        element = document
          .elementFromPoint(point.x / window.devicePixelRatio, point.y / window.devicePixelRatio)
          ?.closest<HTMLElement>('[data-resource-id]');
      const id = element?.dataset.resourceId || ui.foregroundId;
      if (!id) {
        inform('先选中一项待办或正在做的事项');
        return;
      }
      const owner: Owner = {
        kind: element?.dataset.resourceKind === 'todo' ? 'todo' : 'matter',
        id,
      };
      for (const target of event.payload.paths) {
        try {
          await dispatch('resources', 'attach', {
            owner,
            kind: await hostFileKind(target),
            target,
          });
        } catch (error) {
          inform(error);
          break;
        }
      }
    });
  } catch (error) {
    inform(error);
  }
});
async function hostFileKind(path: string) {
  return host.classify(generation.value, path);
}
function outside(event: PointerEvent) {
  if (ui.settingsOpen && !(event.target as HTMLElement).closest('.settings-panel,#btnAppearance'))
    ui.settingsOpen = false;
  if (ui.growthOpen && !(event.target as HTMLElement).closest('.growth-panel,.growth-strip'))
    ui.growthOpen = false;
}
document.addEventListener('pointerdown', outside);
</script>
<template>
  <header
    class="titlebar"
    @pointerdown="
      !($event.target as HTMLElement).closest('button') && $event.button === 0 && host.drag()
    "
  >
    <div class="titlebar-left">
      <span class="brand-mark"><Icon name="check" /></span><strong class="app-title">Catch</strong>
    </div>
    <div class="titlebar-buttons">
      <button
        class="btn-pin"
        :class="{ active: preferences.pinned }"
        :aria-pressed="preferences.pinned"
        title="置顶"
        aria-label="固定置顶"
        @pointerdown.stop
        @click="pin"
      >
        <Icon name="pin" /></button
      ><button title="最小化" aria-label="最小化" @pointerdown.stop @click="host.minimize()">
        <Icon name="minus" /></button
      ><button
        title="收起"
        aria-label="收起"
        :disabled="!ready"
        @pointerdown.stop
        @click="collapse"
      >
        <span class="collapse-dot" />
      </button>
    </div>
  </header>
  <nav class="module-tabs" aria-label="功能导航">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      :class="{ selected: ui.page === tab.id }"
      :aria-selected="ui.page === tab.id"
      @click="page(tab.id)"
    >
      <Icon :name="tab.icon" />{{ tab.name
      }}<i v-if="tab.id === 'current' && current" class="nav-dot" />
    </button>
  </nav>
  <button
    v-if="growth"
    class="growth-strip"
    :class="{ celebrating }"
    :title="growth.selected ? '查看今日成长' : '选择今天的植物'"
    :aria-expanded="!!ui.growthOpen"
    aria-controls="growthPanel"
    @click="toggleGrowth"
  >
    <PlantSprite :plant="growth.plant" :stage="growth.stage" :size="25" /><span
      >今日投入 <strong>{{ human(growth.investedMs) }}</strong></span
    ><span v-if="growth.goalMinutes" class="growth-goal">/ {{ growth.goalMinutes }} 分</span
    ><span v-if="growth.reached" class="growth-check"><Icon name="check" /></span
    ><span class="spacer" /><Icon name="chevron" />
  </button>
  <div v-if="message || state?.storageBlocked" class="activity-message" role="status">
    <span>{{
      message || (state?.storageBlocked ? '保存状态需要检查，请保留文件并重启。' : '')
    }}</span><button v-if="message" aria-label="关闭提示" @click="message = ''">×</button>
  </div>
  <main v-if="ready" class="app-content">
    <Todos v-if="ui.page === 'todos'" /><Current v-else-if="ui.page === 'current'" /><Timeline
      v-else-if="ui.page === 'timeline'"
    /><Trash v-else />
  </main>
  <div v-else class="empty-state loading">
    {{ message ? '暂时无法加载，请检查上方提示。' : '正在打开…' }}
  </div>
  <footer class="app-tools">
    <div class="app-tools-left">
      <button
        id="btnAppearance"
        title="外观"
        aria-label="外观设置"
        :aria-expanded="ui.settingsOpen"
        @click="toggleAppearance"
      >
        <Icon name="sliders" /></button
      ><button
        title="回收站"
        aria-label="打开回收站"
        :aria-pressed="ui.page === 'trash'"
        @click="
          ui.page === 'trash'
            ? page(ui.previousPage)
            : ((ui.previousPage = ui.page), (ui.page = 'trash'))
        "
      >
        <Icon name="trash" />
      </button>
      <button
        :title="ui.page === 'todos' && ui.history ? '返回待办' : '归档'"
        :aria-label="ui.page === 'todos' && ui.history ? '返回待办' : '归档'"
        :aria-pressed="ui.page === 'todos' && ui.history"
        @click="toggleArchive"
      >
        <Icon name="archive" />
      </button>
    </div>
    <span v-if="isolated" class="local-preview" title="使用独立预览数据，不会修改原版记录"
      >本地预览</span
    ><button @click="quit"><Icon name="power" />退出</button>
  </footer>
  <Settings v-if="ui.settingsOpen && ready" /><Editors /><ResourceDialog /><RepeatDialog />
  <Settings
    v-if="ui.growthOpen && ready"
    id="growthPanel"
    mode="growth"
    :style="{ top: `${growthTop}px`, maxHeight: `calc(100% - ${growthTop + 44}px)` }"
  />
  <Modal v-if="quitConfirm" title="暂停并退出？" @close="quitConfirm = false"
    ><div class="editor-body">
      <p class="confirm-name">{{ current?.name }}</p>
    </div>
    <footer class="editor-buttons">
      <button @click="quitConfirm = false">取消</button
      ><button class="primary" :disabled="!!busy" @click="confirmedQuit">暂停并退出</button>
    </footer></Modal
  >
</template>
