<script setup lang="ts" generic="T extends string">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    label: string;
    help?: string;
    options: ReadonlyArray<{ value: T; label: string }>;
    modelValue: T;
    /** Etiket kolonu genişliği px. SettingsPanel 200, ProfileDialog 140. */
    labelWidth?: number;
  }>(),
  { labelWidth: 200 },
);

const emit = defineEmits<{ update: [value: T] }>();

const gridStyle = computed(() => ({
  gridTemplateColumns: `${props.labelWidth}px 1fr`,
}));
</script>

<template>
  <div class="mb-3 grid items-start gap-3" :style="gridStyle">
    <div>
      <div class="text-fg-default">{{ label }}</div>
      <div v-if="help" class="mt-1 text-xs text-fg-subtle">{{ help }}</div>
    </div>
    <div class="flex flex-wrap gap-1">
      <button
        v-for="opt in options"
        :key="opt.value"
        type="button"
        class="rounded border border-border-muted px-2 py-1 text-xs"
        :class="modelValue === opt.value
          ? 'bg-accent-default text-fg-inverse'
          : 'text-fg-muted hover:bg-surface-overlay hover:text-fg-default'"
        @click="emit('update', opt.value)"
      >
        {{ opt.label }}
      </button>
    </div>
  </div>
</template>
