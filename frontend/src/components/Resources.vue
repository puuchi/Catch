<script setup lang="ts">
import { computed, ref } from 'vue';
import Icon from './Icon.vue';
import ActionMenu from './ActionMenu.vue';
import { host } from '../bridge';
import { busy, dispatch, generation, inform, ui } from '../store';
import type { Owner, Resource, ResourceKind } from '../models';
const props = withDefaults(
  defineProps<{ owner: Owner; resources?: Resource[]; readonly?: boolean; compact?: boolean }>(),
  {
    resources: () => [],
    readonly: false,
    compact: false,
  },
);
const container = ref<HTMLElement>();
const key = computed(() => `${props.owner.kind}:${props.owner.id}`),
  expanded = computed({
    get: () => ui.expandedResources?.includes(key.value) || false,
    set: (value) => {
      ui.expandedResources = value
        ? [...(ui.expandedResources || []).filter((id) => id !== key.value), key.value]
        : (ui.expandedResources || []).filter((id) => id !== key.value);
    },
  }),
  adding = ref(false),
  menu = ref<{ resource: Resource; x: number; y: number; trigger?: HTMLElement } | null>(null);
function showMenu(event: MouseEvent, resource: Resource) {
  if (props.readonly) return;
  event.preventDefault();
  const row = (event.currentTarget as HTMLElement).closest('.resource-row');
  const trigger = row?.querySelector<HTMLElement>('.resource-menu-button') || undefined;
  const rect = trigger?.getBoundingClientRect();
  menu.value = {
    resource,
    x: event.type === 'contextmenu' ? event.clientX : rect?.left || 0,
    y: event.type === 'contextmenu' ? event.clientY : rect?.bottom || 0,
    trigger,
  };
}
function menuKey(event: KeyboardEvent, resource: Resource) {
  if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
    event.preventDefault();
    (event.currentTarget as HTMLElement)
      .querySelector<HTMLButtonElement>('.resource-menu-button')
      ?.click();
  }
}
function menuAction(id: string) {
  const resource = menu.value?.resource;
  if (!resource) return;
  if (id === 'modify') void pick(resource.kind, resource);
  if (id === 'remove') void remove(resource);
}
async function pick(kind: ResourceKind, resource?: Resource) {
  adding.value = false;
  if (kind === 'web' || resource) {
    ui.resourceDraft = {
      owner: props.owner,
      kind,
      mode: resource ? 'relocate' : 'attach',
      id: resource?.id,
      target: resource?.target || '',
      title: resource?.title || '',
    };
    return;
  }
  try {
    const paths = await host.pick(generation.value, kind);
    for (const target of paths) {
      await dispatch('resources', 'attach', { owner: props.owner, kind, target });
    }
    if (paths.length) expanded.value = true;
  } catch (error) {
    inform(error);
  }
}
async function open(resource: Resource) {
  try {
    await host.open(generation.value, props.owner, resource.id);
  } catch (error) {
    inform(error);
    expanded.value = true;
    if (!props.readonly) {
      const trigger =
        container.value?.querySelector<HTMLElement>(
          `[data-resource-entry="${CSS.escape(resource.id)}"] .resource-menu-button`,
        ) || undefined;
      const rect = trigger?.getBoundingClientRect();
      menu.value = { resource, x: rect?.left || 8, y: rect?.bottom || 8, trigger };
    }
  }
}
async function remove(resource: Resource) {
  try {
    await dispatch('resources', 'remove', { owner: props.owner, resourceId: resource.id });
    menu.value = null;
  } catch (error) {
    inform(error);
  }
}
</script>
<template>
  <div
    class="resources"
    ref="container"
    :class="{ 'resources-compact': compact }"
    :data-resource-kind="owner.kind"
    :data-resource-id="owner.id"
  >
    <div class="resource-heading">
      <button class="quiet-button" :aria-expanded="expanded" @click="expanded = !expanded">
        <Icon name="clip" />{{
          resources.length
            ? compact
              ? `${resources.length} 份资料`
              : `资料 ${resources.length}`
            : '资料'
        }}</button
      ><button
        v-if="!readonly"
        class="icon-button resource-add"
        title="添加资料"
        aria-label="添加资料"
        :disabled="!!busy"
        @click="adding = !adding"
      >
        <Icon name="plus" />
      </button>
    </div>
    <div v-if="adding && !readonly" class="resource-options">
      <button @click="pick('file')">文件</button><button @click="pick('folder')">文件夹</button
      ><button @click="pick('web')">网页</button>
    </div>
    <div v-if="expanded && resources.length" class="resource-list">
      <div
        v-for="resource in resources"
        :key="resource.id"
        class="resource-row"
        :data-resource-entry="resource.id"
        @contextmenu.stop="showMenu($event, resource)"
        @keydown.stop="menuKey($event, resource)"
      >
        <button class="resource-open" :title="resource.target" @click="open(resource)">
          <Icon :name="resource.kind === 'web' ? 'link' : resource.kind" /><span>{{
            resource.title
          }}</span></button
        ><button
          v-if="!readonly"
          class="icon-button resource-menu-button"
          :aria-label="`${resource.title}的操作`"
          title="资料操作"
          aria-haspopup="menu"
          :aria-expanded="menu?.resource.id === resource.id"
          @click="showMenu($event, resource)"
        >
          ⋯
        </button>
      </div>
    </div>
  </div>
  <ActionMenu
    v-if="menu"
    :x="menu.x"
    :y="menu.y"
    :trigger="menu.trigger"
    label="资料操作"
    :items="[
      { id: 'modify', label: '修改', icon: 'edit', disabled: !!busy },
      { id: 'remove', label: '移除', icon: 'unlink', danger: true, disabled: !!busy },
    ]"
    @select="menuAction"
    @close="menu = null"
  />
</template>
