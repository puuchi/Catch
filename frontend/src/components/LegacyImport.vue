<script setup lang="ts">
import { computed, ref } from 'vue';
import Modal from './Modal.vue';
import { host } from '../bridge';
import { accept, busy, generation, growth, inform, now, state } from '../store';
const opened = ref(false),
  loading = ref(false),
  error = ref('');
const preview = ref<Awaited<ReturnType<typeof host.previewLegacy>>>(null);
const available = computed(
  () =>
    state.value &&
    !state.value.tracking.activities.length &&
    !state.value.tracking.trash.length &&
    !state.value.todos.todos.length &&
    !state.value.todos.archive.length &&
    !state.value.recurrences.templates.length,
);
async function select() {
  loading.value = true;
  error.value = '';
  try {
    preview.value = await host.previewLegacy(generation.value);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}
async function commit() {
  loading.value = true;
  busy.value++;
  try {
    const reply = await host.commitLegacy(generation.value);
    accept(reply.state);
    if (reply.growth) growth.value = reply.growth;
    if (reply.now !== undefined) now.value = reply.now;
    if (!reply.ok) throw Error(reply.error?.message || '导入未完成，请核对记录');
    opened.value = false;
    preview.value = null;
    inform('旧版记录已导入，原文件已保留');
  } catch (cause) {
    error.value = String(cause);
    preview.value = null;
  } finally {
    loading.value = false;
    busy.value--;
  }
}
</script>
<template>
  <div v-if="available" class="settings-section">
    <button class="quiet-button" @click="opened = true">导入旧版记录</button>
  </div>
  <Modal v-if="opened" title="导入旧版记录" @close="!loading && (opened = false)">
    <div class="editor-body">
      <p class="muted small">先退出旧版，再选择它的数据文件夹。只复制记录，旧文件保留。</p>
      <button class="quiet-button" :disabled="loading" @click="select">
        {{ preview ? '重新选择文件夹' : '选择文件夹' }}
      </button>
      <template v-if="preview">
        <p class="muted small import-directory" v-tooltip="preview.directory">
          {{ preview.directory }}
        </p>
        <p>{{ preview.summary.matters }} 件事项 · {{ preview.summary.sessions }} 次投入</p>
        <p>{{ preview.summary.todos }} 条待办 · {{ preview.summary.archived }} 条归档</p>
        <p v-if="preview.summary.trash" class="muted small">
          含 {{ preview.summary.trash }} 条活动回收站记录
        </p>
      </template>
      <p v-if="error" role="alert" class="editor-error">{{ error }}</p>
    </div>
    <footer class="editor-buttons">
      <button :disabled="loading" @click="opened = false">取消</button>
      <button class="primary" :disabled="!preview || loading || !!busy" @click="commit">
        {{ loading ? '处理中…' : '导入' }}
      </button>
    </footer>
  </Modal>
</template>
<style scoped>
.import-directory {
  overflow-wrap: anywhere;
}
</style>
