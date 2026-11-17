<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
const props = defineProps<{ text: string }>();
const container = ref<HTMLElement>();
const fontSize = ref<number>();
let observer: ResizeObserver | undefined;
let context: CanvasRenderingContext2D | null;
let mounted = false;
function fit() {
  const element = container.value;
  if (!element || !context) return;
  const style = getComputedStyle(element);
  const maximum = parseFloat(style.fontSize);
  const spacing = parseFloat(style.letterSpacing) || 0;
  const available = Math.max(0, element.clientWidth - 1);
  // Measure the actual font and current text, not a viewport-based digit estimate.
  let low = 0;
  let high = maximum;
  for (let i = 0; i < 14; i++) {
    const size = (low + high) / 2;
    context.font = `${style.fontWeight} ${size}px ${style.fontFamily}`;
    const width =
      context.measureText(props.text).width + spacing * (size / maximum) * props.text.length;
    if (width <= available) low = size;
    else high = size;
  }
  fontSize.value = Math.floor(low * 100) / 100;
}
onMounted(() => {
  mounted = true;
  context = document.createElement('canvas').getContext('2d');
  observer = new ResizeObserver(fit);
  if (container.value) observer.observe(container.value);
  window.addEventListener('resize', fit);
  document.fonts.addEventListener('loadingdone', fit);
  void document.fonts.ready.then(() => {
    if (mounted) fit();
  });
  fit();
});
watch(() => props.text, fit, { flush: 'post' });
onBeforeUnmount(() => {
  mounted = false;
  observer?.disconnect();
  window.removeEventListener('resize', fit);
  document.fonts.removeEventListener('loadingdone', fit);
});
</script>
<template>
  <div ref="container" class="current-timer" :title="text" :aria-label="`累计投入 ${text}`">
    <span :style="fontSize === undefined ? undefined : { fontSize: `${fontSize}px` }">{{
      text
    }}</span>
  </div>
</template>
