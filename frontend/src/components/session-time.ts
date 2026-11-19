// Calendar arithmetic uses date-only UTC values, not 24-hour local durations (DST).
export function calendarOffset(start: string, end: string): number {
  const from = Date.parse(`${start.slice(0, 10)}T00:00:00Z`);
  const to = Date.parse(`${end.slice(0, 10)}T00:00:00Z`);
  return Number.isFinite(from) && Number.isFinite(to) ? Math.round((to - from) / 86400000) : 0;
}
export function offsetDate(date: string, offset: number): string {
  const value = new Date(`${date}T00:00:00Z`);
  value.setUTCDate(value.getUTCDate() + offset);
  return Number.isFinite(+value) ? value.toISOString().slice(0, 10) : '';
}
export function withDate(value: string, date: string): string {
  return date ? `${date}T${value.split('T')[1] || '00:00:00.000'}` : '';
}
export function withTime(value: string, time: string): string {
  if (!time) return '';
  const original = value.split('T')[1] || '00:00:00.000';
  const seconds = time.length === 5 ? `${time}:${original.slice(6, 8) || '00'}` : time.slice(0, 8);
  // The visible seconds-precision control must not discard stored milliseconds.
  const fraction = original.includes('.') ? original.slice(original.indexOf('.')) : '';
  return `${value.slice(0, 10)}T${seconds}${fraction}`;
}
