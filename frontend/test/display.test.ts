import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import {
  entries,
  gaps,
  duration,
  clock,
  human,
  bounds,
  resizedSegments,
  visibleTodos,
  foreground,
  pausedMatters,
} from '../src/display.ts';
import type { Matter, Todo } from '../src/models.ts';
const require = createRequire(import.meta.url),
  original = require('../../src/domain/tracking/time.js'),
  sessions = require('../../src/domain/tracking/sessions.js');
const fixture = JSON.parse(
  fs.readFileSync(
    new URL('../../crates/catch-core/tests/fixtures/tracking-parity.json', import.meta.url),
    'utf8',
  ),
);

test('urgent todos move above ordinary and deadline-today todos after editing', () => {
  const rows: Todo[] = [
    {
      id: 'ordinary',
      text: '普通',
      notes: '',
      urgent: false,
      deadline: null,
      done: false,
      completedAt: null,
    },
    {
      id: 'today',
      text: '今天截止',
      notes: '',
      urgent: false,
      deadline: '2026-11-17',
      done: false,
      completedAt: null,
    },
    {
      id: 'urgent',
      text: '紧急',
      notes: '',
      urgent: true,
      deadline: null,
      done: false,
      completedAt: null,
    },
    {
      id: 'done',
      text: '完成',
      notes: '',
      urgent: true,
      deadline: null,
      done: true,
      completedAt: '2026-11-17',
    },
  ];
  assert.deepEqual(
    visibleTodos(rows, '2026-11-17').map((row) => row.id),
    ['urgent', 'today', 'ordinary', 'done'],
  );
  const edited = rows.map((row) => (row.id === 'ordinary' ? { ...row, urgent: true } : row));
  assert.deepEqual(
    visibleTodos(edited, '2026-11-17').map((row) => row.id),
    ['ordinary', 'urgent', 'today', 'done'],
  );
  assert.deepEqual(
    rows.map((row) => row.id),
    ['ordinary', 'today', 'urgent', 'done'],
  );
});
test('Vue projections retain original timeline order, clipped duration, progress and gaps', () => {
  for (const scenario of fixture.scenarios)
    for (const step of scenario.steps) {
      for (const date of ['2026-11-13', '2026-11-14']) {
        const actual = entries(step.expected.activities, date, step.now).map((e) => ({
          id: e.matter.id,
          sessionId: e.session.id,
          progress: e.session.progress,
          start: e.start,
          end: e.session.endTime === null && step.now < bounds(date)[1] ? null : e.end,
          duration: e.duration,
        }));
        const expected = original
          .timelineEntries(step.expected.activities, date, step.now)
          .map(
            (e: {
              id: string;
              sessionId: string;
              progress: string;
              startTime: string;
              endTime: string | null;
              segments: [];
            }) => ({
              id: e.id,
              sessionId: e.sessionId,
              progress: e.progress,
              start: Date.parse(e.startTime),
              end: e.endTime === null ? null : Date.parse(e.endTime),
              duration: original.duration(e, step.now, original.dayBounds(date)),
            }),
          );
        assert.deepEqual(actual, expected, `${scenario.name}: ${date}`);
        assert.deepEqual(
          gaps(step.expected.activities, date, step.now),
          original
            .untrackedGaps(step.expected.activities, date, step.now)
            .map((g: { startTime: string; endTime: string }) => ({
              start: Date.parse(g.startTime),
              end: Date.parse(g.endTime),
            })),
        );
      }
      for (const matter of step.expected.activities)
        assert.equal(duration(matter, step.now), original.duration(matter, step.now));
    }
});
test('duration wording and compact session edits preserve original precision and pauses', () => {
  for (const ms of [0, 1000, 59999, 60000, 81000, 3600000, 3661000]) {
    assert.equal(clock(ms), original.clock(ms));
    assert.equal(human(ms), original.humanDuration(ms));
  }
  const session = {
    id: 'a:0',
    startTime: '2026-11-14T01:00:00.123Z',
    endTime: '2026-11-14T01:30:00.456Z',
    progress: '',
    segments: [
      { start: '2026-11-14T01:00:00.123Z', end: '2026-11-14T01:10:00.789Z' },
      { start: '2026-11-14T01:20:00.321Z', end: '2026-11-14T01:30:00.456Z' },
    ],
  };
  for (const [start, end] of [
    [session.startTime, session.endTime],
    ['2026-11-14T01:05:00.000Z', '2026-11-14T01:25:00.000Z'],
    ['2026-11-14T02:00:00.123Z', '2026-11-14T02:30:00.456Z'],
  ])
    assert.deepEqual(
      resizedSegments(session, start, end),
      sessions.resizeSessionSegments(session, start, end),
    );
});

test('closing a completed card keeps paused matters in background until explicitly resumed', () => {
  const rows = fixture.scenarios[0].steps.find(
    (step: { expected: { activities: Matter[] } }) =>
      step.expected.activities.some((matter) => matter.status === 'completed') &&
      step.expected.activities.some((matter) => matter.status === 'paused'),
  ).expected.activities as Matter[];
  const done = rows.find((matter) => matter.status === 'completed')!;
  const paused = rows.find((matter) => matter.status === 'paused')!;
  assert.equal(foreground(rows, done.id)?.id, done.id);
  assert.equal(foreground(rows, null, true), null);
  assert.equal(foreground(rows, done.id, true), null);
  assert.equal(foreground(rows, null)?.id, paused.id);
  assert.equal(
    foreground(
      rows.map((matter) => (matter.id === paused.id ? { ...matter, status: 'running' } : matter)),
      null,
      true,
    )?.id,
    paused.id,
  );
});

test('paused matters sort by latest update without mutating the stored array', () => {
  const sample = fixture.scenarios[0].steps[1].expected.activities[0] as Matter;
  const rows: Matter[] = [
    { ...sample, id: 'old', updatedAt: '2026-11-17T01:00:00Z' },
    { ...sample, id: 'new', updatedAt: '2026-11-17T03:00:00Z' },
    { ...sample, id: 'front', updatedAt: '2026-11-17T04:00:00Z' },
    { ...sample, id: 'middle', updatedAt: '2026-11-17T02:00:00Z' },
  ];
  assert.deepEqual(
    pausedMatters(rows, 'front').map((matter) => matter.id),
    ['new', 'middle', 'old'],
  );
  assert.deepEqual(
    rows.map((matter) => matter.id),
    ['old', 'new', 'front', 'middle'],
  );
});
