import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bounds, duration, entries } from '../src/display.ts';
import {
  adjacentRows,
  detailGroups,
  effortTint,
  focusRows,
  focusSummary,
  HEAT_LEVELS,
  heatLevel,
  isContinuation,
  timelineRows,
} from '../src/timeline.ts';
import type { Matter, Session } from '../src/models.ts';
const iso = (date: string, clock: string) => new Date(`${date}T${clock}`).toISOString();
const date = '2026-11-17';
const now = Date.parse(iso(date, '12:00:00'));
function session(id: string, start: string, end: string): Session {
  return { id, startTime: start, endTime: end, progress: id, segments: [{ start, end }] };
}
function matter(id: string, sessions: Session[]): Matter {
  return {
    id,
    name: id,
    color: '#2679e8',
    status: 'completed',
    todoId: null,
    startTime: sessions[0].startTime,
    endTime: sessions.at(-1)!.endTime,
    sessions,
    segments: sessions.flatMap((session) => session.segments),
    nextSessionIndex: sessions.length,
    createdAt: sessions[0].startTime,
    updatedAt: sessions.at(-1)!.endTime!,
    pauseReason: null,
  };
}
test('timeline interleaves gaps in the same date order, hiding only gaps below ten seconds', () => {
  const items = [
    matter('A', [session('a', iso(date, '09:00:00'), iso(date, '09:10:00'))]),
    matter('B', [session('b', iso(date, '09:10:09'), iso(date, '09:20:00'))]),
    matter('C', [session('c', iso(date, '09:20:10'), iso(date, '09:30:00'))]),
  ];
  const pastRows = timelineRows(items, date, now + 86_400_000);
  assert.deepEqual(
    pastRows.map((row) => row.kind),
    ['entry', 'entry', 'gap', 'entry'],
  );
  assert.equal(
    pastRows.find((row) => row.kind === 'gap')!.end -
      pastRows.find((row) => row.kind === 'gap')!.start,
    10_000,
  );
  const today = timelineRows(items, date, now);
  assert.deepEqual(
    today.map((row) => row.start),
    [...today.map((row) => row.start)].sort((a, b) => b - a),
  );
  assert.deepEqual(
    today.map((row) => row.kind),
    ['gap', 'entry', 'gap', 'entry', 'entry'],
  );
});
test('four intensity bands share exact bar/background thresholds and legend values', () => {
  assert.equal(HEAT_LEVELS.length, 4);
  const values = [0, 899999, 900000, 1799999, 1800000, 3599999, 3600000];
  assert.deepEqual(
    values.map((ms) => heatLevel(ms).opacity),
    [0.25, 0.25, 0.45, 0.45, 0.7, 0.7, 1],
  );
  assert.match(effortTint('#2679e8', 900000)['--effort-bar'], /0\.45\)/);
  assert.match(effortTint('#2679e8', 900000)['--effort-wash'], /0\.075\)/);
  assert.equal(effortTint('invalid', 0)['--effort-color'], '#2e5a49');
});
test('cross-midnight details group visual slices by date while preserving original session identity and effective duration', () => {
  const start = iso('2026-11-16', '23:50:00');
  const end = iso(date, '00:20:00');
  const cross = session('cross', start, end);
  cross.segments = [
    { start, end: iso('2026-11-16', '23:55:00') },
    { start: iso(date, '00:05:00'), end },
  ];
  const item = matter('A', [cross]);
  const before = JSON.stringify(item);
  const groups = detailGroups(item, now);
  assert.deepEqual(
    groups.map((group) => group.date),
    ['2026-11-16', date],
  );
  assert.deepEqual(
    groups.map((group) => group.entries[0].session.id),
    ['cross', 'cross'],
  );
  assert.equal(
    groups.flatMap((group) => group.entries).reduce((sum, entry) => sum + entry.duration, 0),
    duration(item, now),
  );
  assert.equal(focusSummary(item, date, now).count, 1);
  assert.equal(focusSummary(item, date, now).duration, 15 * 60_000);
  assert.equal(JSON.stringify(item), before);
});
test('only mode replaces other matters and gaps with away intervals without turning tracked time into editable gaps', () => {
  const a = matter('A', [
    session('a:0', iso(date, '09:00:00'), iso(date, '09:10:00')),
    session('a:1', iso(date, '09:30:00'), iso(date, '09:40:00')),
  ]);
  const b = matter('B', [session('b:0', iso(date, '09:10:00'), iso(date, '09:20:00'))]);
  const rows = timelineRows([a, b], date, now + 86_400_000);
  const before = JSON.stringify(rows);
  assert.equal(focusRows(rows, 'A', false), rows);
  const focused = focusRows(rows, 'A', true);
  assert.deepEqual(
    focused.map((row) => row.kind),
    ['entry', 'away', 'entry'],
  );
  assert.equal(focused[1].end - focused[1].start, 20 * 60_000);
  assert.equal(JSON.stringify(rows), before);
  assert.deepEqual(focusSummary(a, date, now), { count: 2, duration: 20 * 60_000 });
  const own = entries([a], date, now).sort((x, y) => x.start - y.start);
  assert.equal(isContinuation(own[0]), false);
  assert.equal(isContinuation(own[1]), true);
});
test('adjacent merge candidates come from chronological neighbors, including actual untimed intervals', () => {
  const a = matter('A', [session('a:0', iso(date, '09:00:00'), iso(date, '09:10:00'))]);
  const b = matter('B', [session('b:0', iso(date, '09:20:00'), iso(date, '09:30:00'))]);
  const neighbors = adjacentRows([a, b], date, now, 'b:0');
  assert.deepEqual(
    neighbors.map((row) => row.kind),
    ['gap', 'gap'],
  );
  assert.equal(neighbors[0].start, Date.parse(a.sessions[0].endTime!));
  assert.equal(neighbors[0].end, Date.parse(b.sessions[0].startTime));
  assert.deepEqual(adjacentRows([a, b], date, now, 'missing'), []);
  assert.equal(duration(a, now, bounds(date)), 10 * 60_000);
});
test('only mode coalesces consecutive away rows across hidden short gaps in both display orders', () => {
  const a = matter('A', [session('a', iso(date, '16:44:00'), iso(date, '16:45:00'))]);
  const b = matter('B', [session('b', iso(date, '11:22:00'), iso(date, '11:22:08'))]);
  const c = matter('C', [session('c', iso(date, '11:22:17'), iso(date, '16:44:00'))]);
  const rows = timelineRows([a, b, c], date, now + 86_400_000);
  for (const source of [rows, [...rows].reverse()]) {
    const away = focusRows(source, 'A', true).filter((row) => row.kind === 'away');
    assert.equal(away.length, 1);
    assert.equal(away[0].start, Date.parse(iso(date, '11:22:00')));
    assert.equal(away[0].end, Date.parse(iso(date, '16:44:00')));
  }
});
