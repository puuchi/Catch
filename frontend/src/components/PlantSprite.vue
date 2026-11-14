<script setup lang="ts">
import { computed } from 'vue';
import type { Plant } from '../models';
const props = withDefaults(defineProps<{ plant: Plant; stage: number; size?: number }>(), {
  size: 48,
});
const style = computed(() => {
  const s = Math.max(0, Math.min(4, props.stage));
  if (props.plant === 'sprout') {
    const frame = [15, 20, 29, 42, 60][s];
    return {
      width: `${props.size}px`,
      height: `${props.size}px`,
      backgroundImage: "url('./assets/sprout/growing-plant-sprite.png')",
      backgroundSize: `${props.size * 61}px ${props.size}px`,
      backgroundPosition: `-${props.size * frame}px 0`,
      imageRendering: 'auto' as const,
    };
  }
  const row = { tulip: 1, sunflower: 9, corn: 9, strawberry: 6 }[props.plant],
    column = (
      props.plant === 'corn' || props.plant === 'strawberry' ? [5, 4, 3, 2, 1] : [11, 10, 9, 8, 7]
    )[s];
  return {
    width: `${props.size}px`,
    height: `${props.size}px`,
    backgroundImage: "url('./assets/plants/Crop_Spritesheet.png')",
    backgroundSize: `${props.size * 12}px ${props.size * 10}px`,
    backgroundPosition: `-${props.size * column}px -${props.size * row}px`,
    imageRendering: 'pixelated' as const,
  };
});
</script>
<template><span class="plant-sprite" :style="style" aria-hidden="true" /></template>
