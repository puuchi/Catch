import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type {
  Boot,
  Group,
  IpcRequest,
  Owner,
  Preferences,
  Reply,
  ResourceKind,
  UiState,
} from './models';
export const host = {
  previewLegacy: (generation: number) =>
    invoke<{
      directory: string;
      summary: {
        matters: number;
        sessions: number;
        todos: number;
        archived: number;
        trash: number;
      };
    } | null>('preview_legacy', { generation }),
  commitLegacy: (generation: number) => invoke<Reply>('commit_legacy', { generation }),
  layout: (generation: number, layout: Record<string, unknown>) =>
    invoke('report_layout', { generation, layout }),
  boot: () => invoke<Boot>('boot'),
  command: (
    generation: number,
    group: Group,
    command: Record<string, unknown>,
    expectedRevision: number,
    expectedTodosRevision?: number,
  ) => {
    const request: IpcRequest = {
      requestId: crypto.randomUUID(),
      expectedRevision,
      operation: {
        group,
        command: group === 'recurrence' ? { command, expectedTodosRevision } : command,
      },
    };
    return invoke<Reply>('execute', { generation, request });
  },
  stash: (generation: number, ui: UiState) => invoke('stash_ui', { generation, ui }),
  collapse: (generation: number, ui: UiState) => invoke('collapse', { generation, ui }),
  painted: (generation: number) => invoke('view_painted', { generation }),
  preferences: (generation: number, preferences: Preferences) =>
    invoke('save_preferences', { generation, preferences }),
  pick: (generation: number, kind: ResourceKind) =>
    invoke<string[]>('pick_resources', { generation, kind }),
  open: (generation: number, owner: Owner, id: string) =>
    invoke('open_resource', { generation, owner, id }),
  classify: (generation: number, path: string) =>
    invoke<'file' | 'folder'>('classify_resource', { generation, path }),
  quit: (generation: number) => invoke('quit', { generation }),
  minimize: () => getCurrentWindow().minimize(),
  drag: () => getCurrentWindow().startDragging(),
  on: listen,
};
