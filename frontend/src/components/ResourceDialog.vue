<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import Modal from './Modal.vue';
import ValidationHint from './ValidationHint.vue';
import Icon from './Icon.vue';
import { host } from '../bridge';
import { attachComposerResources, busy, dispatch, generation, ui } from '../store';
const error = ref('');
const modifying = computed(() => ui.resourceDraft?.mode === 'relocate');
const targetLabel = computed(() =>
  ui.resourceDraft?.kind === 'web'
    ? '网页地址'
    : ui.resourceDraft?.kind === 'folder'
      ? '文件夹路径'
      : '文件路径',
);
watch(
  () => ui.resourceDraft,
  () => (error.value = ''),
);
async function browse() {
  const draft = ui.resourceDraft;
  if (!draft || draft.kind === 'web') return;
  try {
    const paths = await host.pick(generation.value, draft.kind);
    if (ui.resourceDraft === draft && paths[0]) draft.target = paths[0];
  } catch (cause) {
    error.value = (cause as Error).message;
  }
}
async function save() {
  const draft = ui.resourceDraft;
  if (!draft || busy.value) return;
  if (modifying.value && !draft.title.trim()) {
    error.value = '先填写资料名称';
    return;
  }
  if (!draft.target.trim()) {
    error.value = `先填写${targetLabel.value}`;
    return;
  }
  try {
    if (draft.owner.kind === 'composer') {
      if (!URL.canParse(draft.target.trim())) throw Error('网页地址无效');
      const url = new URL(draft.target.trim());
      if (
        !['http:', 'https:'].includes(url.protocol) ||
        !url.hostname ||
        url.username ||
        url.password
      )
        throw Error('请使用 http 或 https 网页地址');
      attachComposerResources([
        {
          id: crypto.randomUUID(),
          kind: 'web',
          target: url.href,
          title: draft.title.trim() || url.hostname,
        },
      ]);
      ui.resourceDraft = null;
      return;
    }
    await dispatch(
      'resources',
      draft.mode === 'attach' ? 'attach' : 'relocate',
      draft.mode === 'attach'
        ? { owner: draft.owner, kind: draft.kind, target: draft.target, title: draft.title || null }
        : { owner: draft.owner, resourceId: draft.id, target: draft.target, title: draft.title },
    );
    ui.resourceDraft = null;
  } catch (cause) {
    error.value = String((cause as Error).message);
  }
}
</script>
<template>
  <Modal
    v-if="ui.resourceDraft"
    :title="modifying ? '资料' : '关联网页'"
    @close="ui.resourceDraft = null"
    ><form novalidate @submit.prevent="save">
      <div class="editor-body">
        <template v-if="modifying">
          <label class="stack-field"
            >名称
            <input
              v-model="ui.resourceDraft.title"
              aria-label="资料名称"
              maxlength="200"
              autofocus
              @input="error = ''"
            />
          </label>
          <label class="stack-field"
            >{{ targetLabel }}
            <span class="resource-target-field">
              <input
                v-model="ui.resourceDraft.target"
                :aria-label="targetLabel"
                @input="error = ''"
              />
              <button
                v-if="ui.resourceDraft.kind !== 'web'"
                class="icon-button"
                type="button"
                :title="ui.resourceDraft.kind === 'folder' ? '选择文件夹' : '选择文件'"
                :aria-label="ui.resourceDraft.kind === 'folder' ? '选择文件夹' : '选择文件'"
                :disabled="!!busy"
                @click="browse"
              >
                <Icon name="folder" />
              </button>
            </span>
          </label>
          <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
        </template>
        <template v-else>
          <div class="field-wrap">
            <input
              v-model="ui.resourceDraft.target"
              aria-label="网页地址"
              placeholder="https://…"
              autofocus
              @input="error = ''"
            /><ValidationHint :message="error" @dismiss="error = ''" />
          </div>
          <input
            v-model="ui.resourceDraft.title"
            aria-label="资料名称"
            placeholder="名称（可选）"
            maxlength="200"
          />
        </template>
      </div>
      <footer class="editor-buttons" :class="{ 'resource-editor-buttons': modifying }">
        <button v-if="!modifying" type="button" @click="ui.resourceDraft = null">取消</button
        ><button class="primary" :disabled="!!busy">保存</button>
      </footer>
    </form></Modal
  >
</template>
