export type Status = 'running' | 'paused' | 'completed';
export type Plant = 'tulip' | 'sunflower' | 'corn' | 'strawberry' | 'sprout';
export type ResourceKind = 'file' | 'folder' | 'web';
export interface Resource {
  id: string;
  kind: ResourceKind;
  title: string;
  target: string;
}
export type Owner = { kind: 'todo' | 'matter'; id: string };
export interface Segment {
  start: string;
  end: string | null;
}
export interface Session {
  id: string;
  startTime: string;
  endTime: string | null;
  progress: string;
  segments: Segment[];
}
export interface Matter {
  id: string;
  name: string;
  color: string;
  startTime: string;
  endTime: string | null;
  status: Status;
  segments: Segment[];
  sessions: Session[];
  nextSessionIndex: number;
  todoId: string | null;
  createdAt: string;
  updatedAt: string;
  pauseReason: string | null;
  resources?: Resource[];
}
export interface TrashEntry {
  id: string;
  kind: 'matter' | 'session';
  deletedAt: string;
  activity: Matter;
  session?: Session;
}
export interface TrackingDocument {
  version: number;
  sessionVersion: number;
  revision: number;
  checkpointAt: string | null;
  activities: Matter[];
  trash: TrashEntry[];
}
export interface Todo {
  id: string;
  text: string;
  notes: string;
  urgent: boolean;
  deadline: string | null;
  done: boolean;
  completedAt: string | null;
  deletedAt?: string;
  resources?: Resource[];
  recurrence?: { templateId: string; scheduledOn: string };
}
export interface TodoDocument {
  todos: Todo[];
  archive: Todo[];
  revision: number;
}
export interface TodoFields {
  text: string;
  notes: string;
  urgent: boolean;
  deadline: string | null;
}
export type Schedule =
  | { kind: 'daily' }
  | { kind: 'weekly'; weekdays: number[] }
  | { kind: 'every'; days: number }
  | { kind: 'monthly'; day: number };
export interface RecurrenceTemplate {
  id: string;
  fields: TodoFields;
  resources: Resource[];
  schedule: Schedule;
  startsOn: string;
  nextOn: string;
  lastEmitted: string | null;
  stopped: boolean;
}
export interface RecurrenceDocument {
  version: number;
  revision: number;
  templates: RecurrenceTemplate[];
}
export interface GrowthDocument {
  version: number;
  revision: number;
  goalMinutes: number | null;
  preferredPlant: Plant;
  days: Record<
    string,
    {
      plant: Plant;
      chosenAt: string;
      celebratedAt: string | null;
      goalOnCelebration: number | null;
    }
  >;
}
export interface GrowthView {
  day: string;
  plant: Plant;
  selected: boolean;
  investedMs: number;
  goalMinutes: number | null;
  stage: number;
  reached: boolean;
  celebrated: boolean;
}
export interface Snapshot {
  tracking: TrackingDocument;
  todos: TodoDocument;
  growth: GrowthDocument;
  recurrences: RecurrenceDocument;
  trackingNotice: string | null;
  storageBlocked: boolean;
}
export type Page = 'todos' | 'current' | 'timeline' | 'trash';
export interface EditorDraft {
  kind: 'matter' | 'session' | 'record' | 'todo';
  id?: string;
  activityId?: string;
  sessionId?: string;
  name: string;
  status: Status;
  progress: string;
  start: string;
  end: string;
  notes: string;
  urgent: boolean;
  deadline: string | null;
  scope: 'once' | 'future';
  original?: Session;
}
export interface UiState {
  version: 1;
  page: Page;
  previousPage: Page;
  foregroundId: string | null;
  selectedMatterId: string | null;
  timelineDate: string;
  focusedMatterId: string | null;
  history: boolean;
  activityName: string;
  composer: TodoFields;
  composerExpanded: boolean;
  progressDrafts: Record<string, string>;
  backgroundOpen: boolean;
  editor: EditorDraft | null;
  expandedTodos: string[];
  expandedResources?: string[];
  scroll: Record<string, number>;
  settingsOpen: boolean;
  growthPreview: Plant | null;
  trashFilter: 'all' | 'todo' | 'matter' | 'session';
  resourceDraft: {
    owner: Owner;
    mode: 'attach' | 'relocate';
    id?: string;
    target: string;
    title: string;
    kind: ResourceKind;
  } | null;
  repeatDraft: {
    mode: 'composer' | 'template' | 'convert';
    todoId?: string;
    templateId?: string;
    schedule: Schedule;
    startsOn: string;
  } | null;
  composerSchedule: Schedule | null;
  composerStartsOn: string;
  goalDraft: string | null;
}
export interface Preferences {
  transparency: number;
  pinned: boolean;
}
export interface Boot {
  generation: number;
  state: Snapshot;
  ui: Partial<UiState>;
  now: number;
  growth: GrowthView;
  preferences: Preferences;
  isolated: boolean;
}
export type Group = 'tracking' | 'todos' | 'growth' | 'resources' | 'recurrence';
export interface Reply {
  ok: boolean;
  state: Snapshot;
  growth?: GrowthView;
  now?: number;
  error?: { code: string; message: string };
}
