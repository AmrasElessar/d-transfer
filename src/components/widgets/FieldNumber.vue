<script setup lang="ts">
const props = defineProps<{
  label: string;
  help?: string;
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
}>();

const emit = defineEmits<{ update: [value: number] }>();

function onChange(e: Event) {
  const raw = (e.target as HTMLInputElement).value;
  const step = props.step ?? 1;
  const n = step < 1 ? parseFloat(raw) : parseInt(raw, 10);
  if (Number.isNaN(n)) return;
  const min = props.min ?? 0;
  const max = props.max ?? 1e9;
  emit("update", Math.min(max, Math.max(min, n)));
}
</script>

<template>
  <div class="mb-3 grid grid-cols-[200px_1fr] items-start gap-3">
    <div>
      <div class="text-fg-default">{{ label }}</div>
      <div v-if="help" class="mt-1 text-xs text-fg-subtle">{{ help }}</div>
    </div>
    <input
      type="number"
      class="w-32 rounded border border-border-muted bg-surface-base px-2 py-1 text-right font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step ?? 1"
      @change="onChange"
    />
  </div>
</template>
