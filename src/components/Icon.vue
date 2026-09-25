<script setup lang="ts">
// Icônes au trait 16×16 (reprises de la planche design).
defineProps<{ name: keyof typeof PATHS }>();
</script>

<script lang="ts">
export const PATHS = {
  laptop: '<rect x="3" y="3.5" width="10" height="7" rx="1"/><path d="M1.5 12.5h13"/>',
  cloud: '<path d="M4.5 12.5h7a3 3 0 0 0 .3-6 4 4 0 0 0-7.6 1A2.5 2.5 0 0 0 4.5 12.5z"/>',
  tag: '<path d="M2.5 2.5h5l6 6-5 5-6-6z"/><circle cx="5.5" cy="5.5" r="1"/>',
  box: '<rect x="2.5" y="4" width="11" height="9" rx="1"/><path d="M2.5 7h11M6.5 9.5h3"/>',
  check: '<path d="M3 8.5l3 3 7-7"/>',
  search: '<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5l3 3"/>',
  branch: '<circle cx="4.5" cy="3.5" r="1.5"/><circle cx="4.5" cy="12.5" r="1.5"/><circle cx="11.5" cy="5" r="1.5"/><path d="M4.5 5v6M11.5 6.5c0 3-7 2-7 4.5"/>',
  folder: '<path d="M1.5 4.5a1 1 0 0 1 1-1h3.5l1.5 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z"/>',
  pencil: '<path d="M10.5 2.5l3 3-8 8h-3v-3z"/>',
  copy: '<rect x="5" y="5" width="8.5" height="8.5" rx="1"/><path d="M11 5V3.5a1 1 0 0 0-1-1H3.5a1 1 0 0 0-1 1V10a1 1 0 0 0 1 1H5"/>',
  x: '<path d="M4 4l8 8M12 4l-8 8"/>',
  chev: '<path d="M6 3.5l4.5 4.5L6 12.5"/>',
  warn: '<path d="M8 2l6.5 11.5h-13z"/><path d="M8 6.5v3M8 11.5v.2"/>',
  sun: '<circle cx="8" cy="8" r="3"/><path d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1"/>',
  palette: '<path d="M8 1.5a6.5 6.5 0 1 0 0 13c1 0 1.5-.7 1.5-1.4 0-.9-.8-1.2-.8-2 0-.7.6-1.1 1.3-1.1H12a2.5 2.5 0 0 0 2.5-2.5A6 6 0 0 0 8 1.5z"/><circle cx="5" cy="7" r=".8"/><circle cx="8" cy="4.6" r=".8"/><circle cx="11" cy="6.5" r=".8"/>',
  user: '<circle cx="8" cy="5.5" r="2.5"/><path d="M3 13.5c.8-2.4 2.7-3.5 5-3.5s4.2 1.1 5 3.5"/>',
  file: '<path d="M4 1.5h5l3.5 3.5v9.5H4z"/><path d="M9 1.5V5h3.5"/>',
  globe: '<circle cx="8" cy="8" r="6.5"/><path d="M1.5 8h13M8 1.5c2 2 2.6 4.2 2.6 6.5S10 12.5 8 14.5M8 1.5C6 3.5 5.4 5.7 5.4 8S6 12.5 8 14.5"/>',
  panel: '<rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/><path d="M6 2.5v11"/>',
  plus: '<path d="M8 3v10M3 8h10"/>',
  down: '<path d="M8 2.5v9M4 8l4 4 4-4M3 14h10"/>',
  refresh: '<path d="M13 8a5 5 0 1 1-1.5-3.5"/><path d="M13 2.5v3h-3"/>',
  trash: '<path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.7 9h5.6l.7-9"/>',
  repo: '<path d="M3.5 13V3a1 1 0 0 1 1-1h8v10h-8a1 1 0 0 0-1 1 1 1 0 0 0 1 1h8"/><path d="M6 2v5l1.25-1 1.25 1V2"/>',
  cmd: '<path d="M5.5 5.5h5v5h-5z"/><path d="M5.5 5.5H4a1.5 1.5 0 1 1 1.5-1.5zM10.5 5.5V4a1.5 1.5 0 1 1 1.5 1.5zM10.5 10.5H12a1.5 1.5 0 1 1-1.5 1.5zM5.5 10.5V12A1.5 1.5 0 1 1 4 10.5z"/>',
} as const;
</script>

<template>
  <svg class="i" viewBox="0 0 16 16" aria-hidden="true" v-html="PATHS[name]" />
</template>
