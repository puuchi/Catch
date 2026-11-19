import { bounds, day, entries, gaps, duration } from './display.ts';
import type { Entry } from './display.ts';
import type { Matter } from './models';

export const HEAT_LEVELS = [
  { limit: 15 * 60_000, label: '＜15 分', opacity: 0.25, wash: 0.045 },
  { limit: 30 * 60_000, label: '15–30', opacity: 0.45, wash: 0.075 },
  { limit: 60 * 60_000, label: '30–60', opacity: 0.7, wash: 0.11 },
  { limit: Infinity, label: '≥60 分', opacity: 1, wash: 0.15 },
] as const;
export const heatLevel = (ms: number) =>
  HEAT_LEVELS.find((level) => ms < level.limit) || HEAT_LEVELS[3];
export function effortTint(color: string, ms: number) {
  const safe = /^#[0-9a-f]{6}$/i.test(color) ? color : '#2e5a49';
  const rgb = [1, 3, 5].map((i) => parseInt(safe.slice(i, i + 2), 16)).join(',');
  const level = heatLevel(ms);
  return {
    '--effort-color': safe,
    '--effort-bar': `rgba(${rgb},${level.opacity})`,
    '--effort-wash': `rgba(${rgb},${level.wash})`,
  };
}
export interface EffortRow {
  kind: 'entry';
  key: string;
  start: number;
  end: number;
  entry: Entry;
}
export interface GapRow {
  kind: 'gap' | 'away';
  key: string;
  start: number;
  end: number;
}
export type TimelineRow = EffortRow | GapRow;
export const sessionKey = (sessionId: string, date: string) => `${sessionId}@${date}`;
export function timelineRows(items: Matter[], date: string, now: number): TimelineRow[] {
  const rows: TimelineRow[] = entries(items, date, now).map((entry) => ({
    kind: 'entry',
    key: sessionKey(entry.session.id, date),
    start: entry.start,
    end: entry.end,
    entry,
  }));
  rows.push(
    ...gaps(items, date, now)
      .filter((gap) => gap.end - gap.start >= 30_000)
      .map((gap): GapRow => ({ kind: 'gap', key: `gap:${gap.start}:${gap.end}`, ...gap })),
  );
  const direction = date === day(now) ? -1 : 1;
  return rows.sort((a, b) => direction * (a.start - b.start) || a.key.localeCompare(b.key));
}
export function focusRows(rows: TimelineRow[], matterId: string, only: boolean): TimelineRow[] {
  if (!only) return rows;
  const direction = rows.length > 1 && rows[0].start > rows.at(-1)!.start ? -1 : 1;
  const result: TimelineRow[] = [];
  for (const row of [...rows].sort((a, b) => a.start - b.start)) {
    if (row.kind === 'entry' && row.entry.matter.id === matterId) {
      result.push(row);
      continue;
    }
    const last = result.at(-1);
    // A focused entry is the only separator: hidden short gaps must not split away blocks.
    if (last?.kind === 'away') {
      last.end = Math.max(last.end, row.end);
      last.key = `away:${last.start}:${last.end}`;
    } else
      result.push({
        kind: 'away',
        key: `away:${row.start}:${row.end}`,
        start: row.start,
        end: row.end,
      });
  }
  return direction < 0 ? result.reverse() : result;
}
export function focusSummary(matter: Matter, date: string, now: number) {
  const rows = entries([matter], date, now);
  return {
    count: new Set(rows.map((entry) => entry.session.id)).size,
    duration: duration(matter, now, bounds(date)),
  };
}
export function isContinuation(entry: Entry) {
  return entry.matter.sessions[0]?.id !== entry.session.id;
}
export function detailGroups(matter: Matter, now: number) {
  const dates = new Set<string>();
  for (const session of matter.sessions) {
    dates.add(day(Date.parse(session.startTime)));
    for (const segment of session.segments) {
      let cursor = day(Date.parse(segment.start));
      const stop = day(
        Math.max(Date.parse(segment.start), (segment.end ? Date.parse(segment.end) : now) - 1),
      );
      while (cursor <= stop) {
        dates.add(cursor);
        const next = new Date(`${cursor}T12:00:00`);
        next.setDate(next.getDate() + 1);
        cursor = day(+next);
      }
    }
  }
  return [...dates]
    .sort()
    .map((date) => ({
      date,
      entries: entries([matter], date, now).sort((a, b) => a.start - b.start),
    }))
    .filter((group) => group.entries.length);
}
export function adjacentRows(items: Matter[], date: string, now: number, sessionId: string) {
  const rows = timelineRows(items, date, now).sort((a, b) => a.start - b.start);
  const index = rows.findIndex((row) => row.kind === 'entry' && row.entry.session.id === sessionId);
  if (index < 0) return [];
  return [rows[index - 1], rows[index + 1]].filter((row): row is TimelineRow => !!row);
}
