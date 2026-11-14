<script setup lang="ts">
import { computed, ref } from 'vue';
import Icon from './Icon.vue';
import Modal from './Modal.vue';
import { busy, dispatch, inform, state, ui } from '../store';
import { day, human, duration } from '../display';
const pending = ref<{ group: 'todos' | 'tracking'; id: string; name: string } | null>(null);
const todos = computed(() =>
  ui.trashFilter === 'all' || ui.trashFilter === 'todo'
    ? state.value?.todos.todos.filter((t) => t.deletedAt) || []
    : [],
);
const entries = computed(
  () =>
    state.value?.tracking.trash.filter(
      (e) => ui.trashFilter === 'all' || ui.trashFilter === e.kind,
    ) || [],
);
async function action(group: 'todos' | 'tracking', type: string, id: string) {
  try {
    await dispatch(group, type, { id });
    pending.value = null;
  } catch (error) {
    inform(error);
  }
}
</script>
<template>
  <section class="module-page recycle-page" aria-label="回收站">
    <header class="recycle-heading">
      <button class="icon-button" aria-label="返回" @click="ui.page = ui.previousPage">‹</button
      ><strong>回收站</strong
      ><select v-model="ui.trashFilter" aria-label="回收站类型">
        <option value="all">全部</option>
        <option value="todo">待办</option>
        <option value="matter">事项</option>
        <option value="session">单次记录</option>
      </select>
    </header>
    <div class="recycle-list">
      <article v-for="todo in todos" :key="todo.id" class="recycle-item">
        <strong>{{ todo.text }}</strong>
        <p class="muted small">待办 · {{ todo.deletedAt && day(Date.parse(todo.deletedAt)) }}</p>
        <p v-if="todo.notes" class="todo-notes">{{ todo.notes }}</p>
        <div class="recycle-actions">
          <button :disabled="!!busy" @click="action('todos', 'restore', todo.id)">恢复</button
          ><button @click="pending = { group: 'todos', id: todo.id, name: todo.text }">
            永久删除
          </button>
        </div>
      </article>
      <article v-for="entry in entries" :key="entry.id" class="recycle-item">
        <strong>{{ entry.activity.name }}</strong>
        <p class="muted small">
          {{ entry.kind === 'matter' ? '事项' : '单次记录' }} ·
          {{ day(Date.parse(entry.deletedAt)) }} ·
          {{ human(duration(entry.session || entry.activity, Date.now())) }}
        </p>
        <p v-if="entry.session?.progress" class="todo-notes">{{ entry.session.progress }}</p>
        <div class="recycle-actions">
          <button :disabled="!!busy" @click="action('tracking', 'restore-trash', entry.id)">
            恢复</button
          ><button
            @click="pending = { group: 'tracking', id: entry.id, name: entry.activity.name }"
          >
            永久删除
          </button>
        </div>
      </article>
      <p v-if="!todos.length && !entries.length" class="empty-state">回收站是空的。</p>
    </div>
    <Modal v-if="pending" title="永久删除？" @close="pending = null"
      ><div class="editor-body">
        <p class="confirm-name">{{ pending.name }}</p>
        <p class="muted small">删除后无法恢复。</p>
      </div>
      <footer class="editor-buttons">
        <button @click="pending = null">取消</button
        ><button
          class="danger-button"
          :disabled="!!busy"
          @click="
            action(pending.group, pending.group === 'todos' ? 'delete' : 'delete-trash', pending.id)
          "
        >
          永久删除
        </button>
      </footer></Modal
    >
  </section>
</template>
