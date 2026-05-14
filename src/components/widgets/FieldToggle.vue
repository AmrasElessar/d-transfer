<script setup lang="ts">
defineProps<{
  label: string;
  help?: string;
  modelValue: boolean;
}>();

const emit = defineEmits<{ update: [value: boolean] }>();
</script>

<template>
  <div class="mb-3 grid grid-cols-[200px_1fr] items-start gap-3">
    <div>
      <div class="text-fg-default">{{ label }}</div>
      <div v-if="help" class="mt-1 text-xs text-fg-subtle">{{ help }}</div>
    </div>
    <button
      type="button"
      role="switch"
      :aria-checked="modelValue"
      class="relative inline-flex h-5 w-9 items-center rounded-full border transition-colors"
      :class="modelValue ? 'border-accent-default bg-accent-default' : 'border-border-muted bg-surface-base'"
      @click="emit('update', !modelValue)"
    >
      <span
        class="inline-block size-3.5 rounded-full bg-surface-overlay shadow transition-transform"
        :class="modelValue ? 'translate-x-4' : 'translate-x-0.5'"
      />
    </button>
  </div>
</template>
