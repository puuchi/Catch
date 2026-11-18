import { ref } from 'vue';
import type { Matter, Session } from './models';
import { inputTime } from './display';
import { busy, dispatch, editor, inform, ui } from './store';

// Shared by all entry points and close gestures, not just the mounted form.
export const sessionEditorBusy = ref(false);
export async function openSessionEditor(matter: Matter, session: Session): Promise<boolean> {
  if (busy.value || sessionEditorBusy.value || ui.editor) return false;
  sessionEditorBusy.value = true;
  try {
    let original = session;
    let resumeToken: string | undefined;
    const last = matter.sessions.at(-1)?.id === session.id;
    if (session.endTime === null || (matter.status === 'running' && last)) {
      const hasDraft = Object.prototype.hasOwnProperty.call(ui.progressDrafts, matter.id);
      const progress = ui.progressDrafts[matter.id];
      const snapshot = await dispatch('tracking', 'begin-session-edit', {
        activityId: matter.id,
        sessionId: session.id,
        ...(hasDraft ? { progress } : {}),
      });
      const paused = snapshot.tracking.activities.find((row) => row.id === matter.id);
      const pausedSession = paused?.sessions.find((row) => row.id === session.id);
      if (!paused?.pauseReason || !pausedSession?.endTime) throw Error('未能暂停本次投入，请重试');
      original = pausedSession;
      resumeToken = paused.pauseReason;
      if (hasDraft && ui.progressDrafts[matter.id] === progress)
        delete ui.progressDrafts[matter.id];
    }
    // Do not retain Vue proxies or mutate the snapshot while editing.
    const clone: Session = JSON.parse(JSON.stringify(original));
    ui.editor = editor({
      kind: 'session',
      activityId: matter.id,
      sessionId: clone.id,
      name: matter.name,
      progress: clone.progress,
      start: inputTime(clone.startTime),
      end: inputTime(clone.endTime!),
      original: clone,
      ...(resumeToken ? { resumeToken } : {}),
    });
    return true;
  } catch (error) {
    inform(error);
    return false;
  } finally {
    sessionEditorBusy.value = false;
  }
}

export async function closeEditor(): Promise<void> {
  if (busy.value || sessionEditorBusy.value) return;
  const draft = ui.editor;
  if (!draft) return;
  if (!draft.resumeToken) {
    ui.editor = null;
    return;
  }
  sessionEditorBusy.value = true;
  try {
    await dispatch('tracking', 'finish-session-edit', {
      activityId: draft.activityId,
      sessionId: draft.sessionId,
      editToken: draft.resumeToken,
      cancel: true,
    });
    if (ui.editor === draft) ui.editor = null;
  } finally {
    // Failures deliberately propagate to Editors; keep the draft and edit token.
    sessionEditorBusy.value = false;
  }
}
