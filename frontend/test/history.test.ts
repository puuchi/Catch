import { test } from 'node:test';
import assert from 'node:assert/strict';
import { historyDate, inDateRange } from '../src/history.ts';
test('history filters include both boundaries and allow either open end', () => {
  for (const date of ['2026-11-16', '2026-11-17'])
    assert.equal(inDateRange(date, '2026-11-16', '2026-11-17'), true);
  assert.equal(inDateRange('2026-11-15', '2026-11-16', ''), false);
  assert.equal(inDateRange('2026-11-18', '', '2026-11-17'), false);
  assert.equal(inDateRange('2026-11-18', '2026-11-17', ''), true);
});
test('invalid ranges show no misleading results and missing dates are excluded only when filtering', () => {
  assert.equal(inDateRange('2026-11-17', '2026-11-18', '2026-11-16'), false);
  assert.equal(inDateRange(null, '', ''), true);
  assert.equal(inDateRange(null, '2026-11-17', ''), false);
  assert.equal(historyDate('not a date'), '');
  assert.equal(historyDate('2026-11-17'), '2026-11-17');
});
test('deletion timestamp filters use the same local day as the displayed date', () => {
  const stamp = new Date(2026, 10, 17, 23, 59, 59).toISOString();
  assert.equal(historyDate(stamp), '2026-11-17');
  assert.equal(inDateRange(stamp, '2026-11-17', '2026-11-17'), true);
});
