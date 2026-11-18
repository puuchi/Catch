import type { App, ObjectDirective } from 'vue';

type TooltipValue = string | null | undefined;

export function installTooltips(app: App) {
  const values = new WeakMap<HTMLElement, TooltipValue>();
  const cleanups = new WeakMap<HTMLElement, () => void>();
  let tooltip: HTMLElement | undefined;
  let owner: HTMLElement | undefined;
  let waiting: HTMLElement | undefined;
  let pending: ReturnType<typeof setTimeout> | undefined;
  let dismissed = false;

  function hide() {
    clearTimeout(pending);
    waiting = undefined;
    if (owner && tooltip) {
      const ids = (owner.getAttribute('aria-describedby') || '')
        .split(/\s+/)
        .filter((id) => id && id !== tooltip!.id);
      if (ids.length) owner.setAttribute('aria-describedby', ids.join(' '));
      else owner.removeAttribute('aria-describedby');
    }
    tooltip?.hidePopover();
    owner = undefined;
  }

  function show(element: HTMLElement) {
    hide();
    const text = values.get(element);
    if (!text || !element.isConnected) return;
    if (!tooltip) {
      tooltip = document.createElement('div');
      tooltip.id = 'catch-tooltip';
      tooltip.className = 'app-tooltip';
      tooltip.setAttribute('role', 'tooltip');
      tooltip.setAttribute('popover', 'manual');
      document.body.append(tooltip);
    }
    owner = element;
    tooltip.textContent = text;
    tooltip.showPopover();
    const anchor = element.getBoundingClientRect();
    const box = tooltip.getBoundingClientRect();
    const margin = 8;
    const gap = 6;
    const below = anchor.bottom + gap;
    const top =
      below + box.height <= window.innerHeight - margin ? below : anchor.top - gap - box.height;
    tooltip.style.left = `${Math.max(margin, Math.min(anchor.left, window.innerWidth - box.width - margin))}px`;
    tooltip.style.top = `${Math.max(margin, Math.min(top, window.innerHeight - box.height - margin))}px`;
    const describedBy = element.getAttribute('aria-describedby');
    element.setAttribute('aria-describedby', [describedBy, tooltip.id].filter(Boolean).join(' '));
  }

  function key(event: KeyboardEvent) {
    dismissed = event.key === 'Escape';
    if (dismissed) hide();
  }

  const directive: ObjectDirective<HTMLElement, TooltipValue> = {
    mounted(element, binding) {
      values.set(element, binding.value);
      const enter = (event: PointerEvent) => {
        if (event.pointerType === 'touch') return;
        dismissed = false;
        clearTimeout(pending);
        waiting = element;
        pending = setTimeout(() => show(element), 300);
      };
      const focus = () => {
        if (!dismissed && element.matches(':focus-visible')) show(element);
      };
      const leave = () => {
        if (waiting === element) {
          clearTimeout(pending);
          waiting = undefined;
        }
        if (owner === element) hide();
      };
      element.addEventListener('pointerenter', enter);
      element.addEventListener('pointerleave', leave);
      element.addEventListener('focus', focus);
      element.addEventListener('blur', leave);
      cleanups.set(element, () => {
        leave();
        element.removeEventListener('pointerenter', enter);
        element.removeEventListener('pointerleave', leave);
        element.removeEventListener('focus', focus);
        element.removeEventListener('blur', leave);
      });
    },
    updated(element, binding) {
      values.set(element, binding.value);
      if (owner === element && binding.value !== binding.oldValue) show(element);
    },
    beforeUnmount(element) {
      cleanups.get(element)?.();
      cleanups.delete(element);
      values.delete(element);
    },
  };

  app.directive('tooltip', directive);
  document.addEventListener('keydown', key, true);
  document.addEventListener('pointerdown', hide, true);
  document.addEventListener('scroll', hide, true);
  window.addEventListener('resize', hide);
  window.addEventListener('blur', hide);
  app.onUnmount(() => {
    hide();
    tooltip?.remove();
    document.removeEventListener('keydown', key, true);
    document.removeEventListener('pointerdown', hide, true);
    document.removeEventListener('scroll', hide, true);
    window.removeEventListener('resize', hide);
    window.removeEventListener('blur', hide);
  });
}
