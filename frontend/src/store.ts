import { reactive, ref, shallowRef, watch } from 'vue';
import { host } from './bridge';
import { day, foreground } from './display';
import type {
  Boot,
  Group,
  GrowthView,
  Snapshot,
  UiState,
  EditorDraft,
  Preferences,
  Owner,
  Resource,
} from './models';

const defaults = (): UiState => ({
  version: 1,
  page: 'todos',
  previousPage: 'todos',
  foregroundId: null,
  foregroundClosed: false,
  selectedMatterId: null,
  timelineDate: day(),
  focusedMatterId: null,
  history: false,
  activityName: '',
  composer: { text: '', notes: '', urgent: false, deadline: null },
  composerResources: [],
  composerExpanded: false,
  progressDrafts: {},
  backgroundOpen: true,
  editor: null,
  expandedTodos: [],
  collapsedTodos: [],
  expandedTodoNotes: [],
  scroll: {},
  settingsOpen: false,
  growthOpen: false,
  growthPreview: null,
  trashFilter: 'all',
  resourceDraft: null,
  repeatDraft: null,
  composerSchedule: null,
  composerStartsOn: day(),
  goalDraft: null,
});
export const ui = reactive<UiState>(defaults());
export const state = shallowRef<Snapshot>();
export const growth = ref<GrowthView>();
export const now = ref(Date.now()),
  generation = ref(0),
  busy = ref(0),
  message = ref(''),
  celebrating = ref(false),
  isolated = ref(true);
export const preferences = reactive<Preferences>({ transparency: 45, pinned: true });
let initialized = false,
  closing = false,
  saveTimer: ReturnType<typeof setTimeout> | undefined,
  saveQueue = Promise.resolve();
let messageTimer: ReturnType<typeof setTimeout> | undefined;
export function inform(error: unknown, duration = 0) {
  clearTimeout(messageTimer);
  message.value = error instanceof Error ? error.message : String(error);
  if (duration > 0) messageTimer = setTimeout(() => (message.value = ''), duration);
}
export function attachComposerResources(resources: Resource[]) {
  const current = ui.composerResources || [];
  const identity = (resource: Resource) =>
    resource.kind === 'web'
      ? new URL(resource.target).href
      : resource.target.replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase();
  const keys = new Set(current.map(identity));
  const added = resources.filter((resource) => {
    const key = identity(resource);
    if (keys.has(key)) return false;
    keys.add(key);
    return true;
  });
  if (current.length + added.length > 32) throw Error('每项最多关联 32 份资料');
  ui.composerResources = [...current, ...added];
}
export function accept(next: Snapshot) {
  const previous = state.value;
  state.value = previous
    ? {
        ...next,
        tracking:
          next.tracking.revision >= previous.tracking.revision ? next.tracking : previous.tracking,
        todos: next.todos.revision >= previous.todos.revision ? next.todos : previous.todos,
        growth: next.growth.revision >= previous.growth.revision ? next.growth : previous.growth,
        recurrences:
          next.recurrences.revision >= previous.recurrences.revision
            ? next.recurrences
            : previous.recurrences,
      }
    : next;
  const current = foreground(state.value.tracking.activities, ui.foregroundId, ui.foregroundClosed);
  if (current) {
    ui.foregroundId = current.id;
    ui.foregroundClosed = false;
  }
}
export const snapshotUi = () => JSON.parse(JSON.stringify(ui)) as UiState;
export function flushUi() {
  clearTimeout(saveTimer);
  const value = snapshotUi(),
    token = generation.value;
  const save = saveQueue.catch(() => {}).then(() => host.stash(token, value));
  saveQueue = save.then(
    () => {},
    () => {},
  );
  return save;
}
watch(
  ui,
  () => {
    if (!initialized || closing) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void flushUi().catch(inform);
    }, 350);
  },
  { deep: true },
);
export async function dispatch(group: Group, type: string, payload: Record<string, unknown> = {}) {
  if (!state.value) throw Error('正在加载');
  const document =
    group === 'tracking' || group === 'finish'
      ? state.value.tracking
      : group === 'todos'
        ? state.value.todos
        : group === 'growth'
          ? state.value.growth
          : group === 'recurrence'
            ? state.value.recurrences
            : (payload.owner as Owner)?.kind === 'todo'
              ? state.value.todos
              : state.value.tracking;
  busy.value++;
  try {
    const command =
      group === 'finish'
        ? { ...payload, expectedTodosRevision: state.value.todos.revision }
        : ['archive', 'checkpoint'].includes(type)
          ? { type }
          : { type, payload };
    const reply = await host.command(
      generation.value,
      group,
      command,
      document.revision,
      state.value.todos.revision,
    );
    accept(reply.state);
    if (reply.growth) growth.value = reply.growth;
    if (reply.now !== undefined) now.value = reply.now;
    if (!reply.ok) throw Error(reply.error?.message || '操作未完成，请核对记录');
    return reply.state;
  } finally {
    busy.value--;
  }
}
export async function collapse() {
  if (closing) return;
  closing = true;
  clearTimeout(saveTimer);
  try {
    await savePreferences();
    await flushUi();
    await host.collapse(generation.value, snapshotUi());
  } catch (error) {
    closing = false;
    inform(error);
  }
}
export async function savePreferences() {
  try {
    await host.preferences(generation.value, { ...preferences });
  } catch (error) {
    inform(error);
  }
}
export async function initialize() {
  const boot: Boot = await host.boot();
  generation.value = boot.generation;
  Object.assign(ui, defaults(), boot.ui, {
    composer: { ...defaults().composer, ...boot.ui.composer },
  });
  Object.assign(preferences, boot.preferences);
  isolated.value = boot.isolated;
  now.value = boot.now;
  growth.value = boot.growth;
  accept(boot.state);
  initialized = true;
  await host.on<Snapshot>('catch:state', (event) => accept(event.payload));
  await host.on<{ now: number; growth: GrowthView; celebrate: boolean }>('catch:tick', (event) => {
    now.value = event.payload.now;
    growth.value = event.payload.growth;
    if (event.payload.celebrate) {
      celebrating.value = true;
      setTimeout(() => (celebrating.value = false), 2200);
    }
  });
  await host.on<string>('catch:error', (event) => inform(event.payload));
}
export function editMatter(id: string) {
  const matter = state.value?.tracking.activities.find((m) => m.id === id);
  if (!matter) return;
  ui.editor = editor({ kind: 'matter', id, name: matter.name, status: matter.status });
}
export function editor(value: Partial<EditorDraft> & Pick<EditorDraft, 'kind'>): EditorDraft {
  return {
    name: '',
    status: 'completed',
    progress: '',
    start: '',
    end: '',
    notes: '',
    urgent: false,
    deadline: null,
    scope: 'once',
    ...value,
  };
}
