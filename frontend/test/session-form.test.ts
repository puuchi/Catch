import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { calendarOffset, offsetDate, withDate, withTime } from '../src/components/session-time.ts';
import { inputTime, resizedSegments, duration } from '../src/display.ts';
import type { Matter, Session } from '../src/models.ts';

const require = createRequire(import.meta.url);
const { reactive, ref } = require('vue');
const ts = require('typescript');
const source = ts.transpileModule(
  readFileSync(new URL('../src/session-editor.ts', import.meta.url), 'utf8'),
  {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  },
).outputText;
function harness(session: Session) {
  const matter = {
    id: 'm',
    name: '事项',
    status: session.endTime ? 'completed' : 'running',
    sessions: [session],
  } as Matter;
  const ui = reactive({ editor: null, progressDrafts: {} as Record<string, string> });
  const calls: { type: string; payload: Record<string, unknown> }[] = [];
  const errors: unknown[] = [];
  let fail = false;
  let wait: Promise<void> | undefined;
  const store = {
    ui,
    busy: ref(0),
    inform: (error: unknown) => errors.push(error),
    editor: (value: unknown) => value,
    async dispatch(_group: string, type: string, payload: Record<string, unknown>) {
      calls.push({ type, payload });
      if (wait) await wait;
      if (fail) throw Error('模拟写入失败');
      const original = {
        ...session,
        endTime: '2026-11-17T03:00:00.987Z',
        progress: payload.progress ?? session.progress,
        segments: session.segments.map((s) => ({ ...s, end: s.end || '2026-11-17T03:00:00.987Z' })),
      };
      return {
        tracking: {
          activities: [{ ...matter, pauseReason: 'unique-edit-token', sessions: [original] }],
        },
      };
    },
  };
  const exports: Record<string, any> = {};
  new Function('require', 'exports', source)((id: string) => {
    if (id === 'vue') return { ref };
    if (id === './display') return { inputTime };
    if (id === './store') return store;
    throw Error(`Unexpected dependency ${id}`);
  }, exports);
  return {
    ...exports,
    matter,
    ui,
    calls,
    errors,
    store,
    fail: (value: boolean) => {
      fail = value;
    },
    wait: (value?: Promise<void>) => {
      wait = value;
    },
  };
}
const closed: Session = {
  id: 's',
  startTime: '2026-11-14T01:02:03.123Z',
  endTime: '2026-11-17T03:00:00.987Z',
  progress: '旧进展',
  segments: [
    { start: '2026-11-14T01:02:03.123Z', end: '2026-11-14T02:02:03.456Z' },
    { start: '2026-11-17T02:00:00.789Z', end: '2026-11-17T03:00:00.987Z' },
  ],
};
const running: Session = {
  ...closed,
  endTime: null,
  segments: [{ start: closed.startTime, end: null }],
};

test('date/time controls preserve milliseconds and arbitrary calendar spans without DST truncation', () => {
  const start = '2026-03-07T23:12:34.123',
    end = '2026-03-10T01:22:33.987';
  assert.equal(calendarOffset(start, end), 3);
  assert.equal(offsetDate('2026-03-07', 3), '2026-03-10');
  assert.equal(offsetDate('2026-12-31', 1), '2027-01-01');
  assert.equal(withDate(start, '2026-11-01'), '2026-11-01T23:12:34.123');
  assert.equal(withTime(start, '08:09:10'), '2026-03-07T08:09:10.123');
  assert.equal(withTime(start, '08:09'), '2026-03-07T08:09:34.123');
  assert.equal(withTime(start, '23:12:34'), start);
  assert.equal(
    withDate(end, offsetDate('2026-11-01', calendarOffset(start, end))),
    '2026-11-04T01:22:33.987',
  );
});
test('progress-only edits preserve exact endpoints and duration excludes paused gaps', () => {
  const start = new Date(inputTime(closed.startTime)).toISOString();
  const end = new Date(inputTime(closed.endTime!)).toISOString();
  assert.equal(start, closed.startTime);
  assert.equal(end, closed.endTime);
  assert.deepEqual(resizedSegments(closed, start, end), closed.segments);
  assert.equal(duration(closed, 0), 7200531);
  assert.ok(duration(closed, 0) < Date.parse(end) - Date.parse(start));
});
test('closed sessions open directly with a detached original and cancel without commands', async () => {
  const h = harness(closed);
  assert.equal(await h.openSessionEditor(h.matter, closed), true);
  assert.equal(h.calls.length, 0);
  assert.deepEqual(h.ui.editor.original, closed);
  h.ui.editor.original.segments[0].start = 'changed';
  assert.notEqual(closed.segments[0].start, 'changed');
  assert.equal(h.ui.editor.resumeToken, undefined);
  await h.closeEditor();
  assert.equal(h.ui.editor, null);
  assert.equal(h.calls.length, 0);
});
test('running begin atomically includes even an empty progress draft, then cancel uses its token', async () => {
  const h = harness(running);
  h.ui.progressDrafts.m = '';
  assert.equal(await h.openSessionEditor(h.matter, running), true);
  assert.deepEqual(h.calls[0], {
    type: 'begin-session-edit',
    payload: { activityId: 'm', sessionId: 's', progress: '' },
  });
  assert.equal(Object.hasOwn(h.ui.progressDrafts, 'm'), false);
  assert.equal(h.ui.editor.original.endTime, '2026-11-17T03:00:00.987Z');
  assert.equal(h.ui.editor.resumeToken, 'unique-edit-token');
  await h.closeEditor();
  assert.deepEqual(h.calls[1], {
    type: 'finish-session-edit',
    payload: { activityId: 'm', sessionId: 's', editToken: 'unique-edit-token', cancel: true },
  });
  assert.equal(h.ui.editor, null);
});
test('begin failure informs, preserves progress draft and never opens the form', async () => {
  const h = harness(running);
  h.ui.progressDrafts.m = '未保存';
  h.fail(true);
  assert.equal(await h.openSessionEditor(h.matter, running), false);
  assert.equal(h.ui.progressDrafts.m, '未保存');
  assert.equal(h.ui.editor, null);
  assert.equal(h.errors.length, 1);
  assert.equal(h.sessionEditorBusy.value, false);
});
test('cancel failure propagates and retains form/token for retry', async () => {
  const h = harness(running);
  await h.openSessionEditor(h.matter, running);
  const draft = h.ui.editor;
  h.fail(true);
  await assert.rejects(h.closeEditor(), /模拟写入失败/);
  assert.equal(h.ui.editor, draft);
  assert.equal(h.ui.editor.resumeToken, 'unique-edit-token');
  assert.equal(h.sessionEditorBusy.value, false);
  h.fail(false);
  await h.closeEditor();
  assert.equal(h.ui.editor, null);
});
test('busy/local locks reject duplicate opens and duplicate cancel commands', async () => {
  const h = harness(running);
  h.store.busy.value = 1;
  assert.equal(await h.openSessionEditor(h.matter, running), false);
  h.store.busy.value = 0;
  let release!: () => void;
  h.wait(
    new Promise<void>((resolve) => {
      release = resolve;
    }),
  );
  const opening = h.openSessionEditor(h.matter, running);
  assert.equal(await h.openSessionEditor(h.matter, running), false);
  release();
  await opening;
  assert.equal(h.calls.length, 1);
  h.wait(
    new Promise<void>((resolve) => {
      release = resolve;
    }),
  );
  const closing = h.closeEditor();
  await h.closeEditor();
  assert.equal(h.calls.length, 2);
  release();
  await closing;
  assert.equal(h.ui.editor, null);
});
test('begin without a draft omits progress and does not remove a newer in-flight draft', async () => {
  const h = harness(running);
  await h.openSessionEditor(h.matter, running);
  assert.equal(Object.hasOwn(h.calls[0].payload, 'progress'), false);
  await h.closeEditor();
  h.ui.progressDrafts.m = '原草稿';
  let release!: () => void;
  h.wait(
    new Promise<void>((resolve) => {
      release = resolve;
    }),
  );
  const opening = h.openSessionEditor(h.matter, running);
  h.ui.progressDrafts.m = '较新草稿';
  release();
  await opening;
  assert.equal(h.ui.progressDrafts.m, '较新草稿');
});
