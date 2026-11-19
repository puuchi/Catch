import model from './model.mjs';

const origin = window.location.origin;
const actualStart = Date.now();
const exampleStart = new Date(actualStart);
exampleStart.setHours(16, 20, 0, 0);
const preview = model.createPreview({ now: () => +exampleStart + (Date.now() - actualStart) });
const listeners = new Map();
const send = (type, payload) => window.parent.postMessage({ catchDemo: type, payload }, origin);
const ignore = async () => {};

function emit(event, payload) {
  for (const listener of listeners.get(event) || []) listener({ payload });
}

function tick() {
  const growth = preview.growth();
  emit('catch:tick', { now: preview.demo.now(), growth, celebrate: false });
  send('activity', { growth, running: !!preview.demo.current, repeatDue: !!preview.repeatDue() });
}

window.addEventListener('message', (event) => {
  if (event.origin !== origin || event.source !== window.parent) return;
  const { catchDemo, payload } = event.data || {};
  if (catchDemo === 'page') {
    const label = { todos: '待办', current: '正在做', timeline: '时间轴' }[payload];
    const tab = [...document.querySelectorAll('.module-tabs button')].find(
      (item) => item.textContent.trim() === label,
    );
    tab?.click();
    tab?.focus();
  }
  if (catchDemo === 'growth') {
    const strip = document.querySelector('.growth-strip');
    if (strip?.getAttribute('aria-expanded') === 'false') strip.click();
    strip?.focus();
  }
  if (catchDemo === 'advance-growth') {
    preview.demo.simulateGrowth();
    tick();
  }
  if (catchDemo === 'advance-repeat') {
    emit('catch:state', preview.simulateRepeat());
    tick();
  }
});

export const host = {
  boot: async () => ({
    generation: 1,
    state: preview.snapshot(),
    growth: preview.growth(),
    now: preview.demo.now(),
    ui: { foregroundId: preview.demo.focusId, timelineDate: preview.demo.today },
    preferences: { transparency: 8, pinned: true },
    isolated: false,
  }),
  command: async (_generation, group, command) => {
    const reply = preview.command(group, command);
    tick();
    return reply;
  },
  on: async (event, listener) => {
    const list = listeners.get(event) || new Set();
    listeners.set(event, list);
    list.add(listener);
    return () => list.delete(listener);
  },
  layout: ignore,
  painted: async () => { tick(); send('ready'); },
  stash: ignore,
  preferences: ignore,
  collapse: async () => { send('collapse'); return false; },
  minimize: async () => send('collapse'),
  drag: ignore,
  quit: async () => {
    preview.demo.tracking.pause();
    emit('catch:state', preview.snapshot());
    tick();
    send('collapse');
  },
  pick: async (_generation, kind) => kind === 'folder'
    ? ['C:\\社区书架\\归还记录']
    : ['C:\\社区书架\\开放时间.pdf'],
  open: async (_generation, owner, id) => {
    const resource = preview.resources(owner.id).find((item) => item.id === id);
    if (resource?.kind === 'web') {
      const url = new URL(resource.target);
      if (url.protocol === 'https:' || url.protocol === 'http:') window.open(url.href, '_blank', 'noopener,noreferrer');
    } else if (resource) {
      emit('catch:error', '这是演示资料。桌面版可打开本机文件。');
    }
  },
};

const interval = window.setInterval(tick, 1000);
window.addEventListener('pagehide', () => window.clearInterval(interval), { once: true });
