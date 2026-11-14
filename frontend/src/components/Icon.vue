<script setup lang="ts">
defineProps<{ name: string }>();
const paths: Record<string, string> = {
  trash: 'M3 6h18M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7M14 10v7',
  plus: 'M12 5v14M5 12h14',
  close: 'M6 6l12 12M18 6L6 18',
  check: 'M5 12l4 4L19 6',
  pin: 'M8 3h8M9 3v5l-3 4v2h12v-2l-3-4V3M12 14v7',
  minus: 'M5 12h14',
  list: 'M9 6h11M9 12h11M9 18h11M3 6h1M3 12h1M3 18h1',
  timer: 'M12 8v5l3 2M9 2h6M12 2v3M20 5l-2 2M20 13a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
  timeline: 'M5 3v18M5 6h4M5 12h4M5 18h4M13 5h7M13 11h7M13 17h7',
  play: 'M8 5l11 7-11 7z',
  pause: 'M8 5v14M16 5v14',
  stop: 'M6 6h12v12H6Z',
  next: 'M3 12h14M11 6l6 6-6 6M21 5v14',
  edit: 'M15 4l5 5M4 20l2-7L16 3l5 5-10 10-7 2',
  chevron: 'M6 9l6 6 6-6',
  sliders: 'M4 7h16M4 17h16M8 4v6M16 14v6',
  power: 'M12 3v9M6 5a9 9 0 1 0 12 0',
  flag: 'M5 21V3M5 4c5-5 9 5 14 0v10c-5 5-9-5-14 0',
  calendar: 'M4 5h16v16H4zM8 3v4M16 3v4M4 10h16',
  clip: 'M8 12l6-6a3 3 0 0 1 4 4l-8 8a5 5 0 0 1-7-7l8-8',
  repeat: 'M4 8h15l-3-3M20 16H5l3 3M19 8v4M5 16v-4',
  folder: 'M3 6h7l2 3h9v12H3z',
  link: 'M10 14l4-4M8 16l-2 2a4 4 0 0 1-6-6l4-4M16 8l2-2a4 4 0 0 1 6 6l-4 4',
  file: 'M6 3h8l4 4v14H6zM14 3v5h5',
  arrow: 'M6 18L18 6M7 6h11v11',
  broom: 'M14 3l-5 9M7 11l7 4-4 6-7-4 4-6M5 16l6 3',
  info: 'M12 11v6M12 7h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0',
};
</script>
<template>
  <svg
    class="ui-icon"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name] || paths.info" />
  </svg>
</template>
