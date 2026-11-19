import { day } from './display.ts';
export function historyDate(value?: string | null): string {
  if (!value) return '';
  if (/^\d{4}-\d{2}-\d{2}$/.test(value)) return value;
  const time = Date.parse(value);
  return Number.isFinite(time) ? day(time) : '';
}
export function inDateRange(value: string | null | undefined, from: string, to: string): boolean {
  if (from && to && from > to) return false;
  const date = historyDate(value);
  return (!from && !to) || (!!date && (!from || date >= from) && (!to || date <= to));
}
