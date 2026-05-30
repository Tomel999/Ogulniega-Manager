<script setup>
defineProps({
  label: { type: String, required: true },
  disabled: { type: Boolean, default: false },
});
defineEmits(['click']);
</script>

<template>
  <div
    :class="['button-card', { disabled }]"
    :aria-disabled="disabled"
    @click="!disabled && $emit('click')"
  >
    <div class="icon">
      <slot />
    </div>
    <span class="button-label">{{ label }}</span>
  </div>
</template>

<style scoped>
.button-card.disabled {
  opacity: 0.45;
  cursor: not-allowed;
  filter: grayscale(0.65);
  transform: none;
}

.button-card.disabled:hover {
  background-color: var(--card-bg);
  transform: none;
  box-shadow: 0 4px 6px rgba(0,0,0,0.1);
}
</style>
