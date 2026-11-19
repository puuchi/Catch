import { test } from 'node:test';
import assert from 'node:assert/strict';
import { calendarDays, dateValue, parseDate } from '../src/calendar.ts';
test('date values are locale-independent and invalid calendar dates are rejected', () => {
  assert.equal(dateValue(parseDate('2026-11-18')!), '2026-11-18');
  assert.ok(parseDate('2024-02-29'));
  for (const value of [
    '2026-02-29',
    '2026-11-31',
    '2026-13-01',
    '2026-00-01',
    '2026-11-00',
    '18/11/2026',
    '',
  ])
    assert.equal(parseDate(value), null);
});
test('calendar uses Monday-first six weeks and retains cross-month dates', () => {
  const days = calendarDays(2026, 10);
  assert.equal(days.length, 42);
  assert.equal(days[0].value, '2026-10-26');
  assert.equal(days[6].value, '2026-11-01');
  assert.equal(days.filter((d) => !d.outside).length, 30);
  assert.equal(calendarDays(2024, 1).filter((d) => !d.outside).length, 29);
  assert.equal(calendarDays(2026, 11).at(-1)!.value, '2027-01-10');
});
