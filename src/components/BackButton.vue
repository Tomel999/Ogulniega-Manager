<script setup>
defineEmits(['click']);
</script>

<template>
  <div class="back-row">
    <div class="back-button" @click="$emit('click')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
      <span>{{ $t('common.back') }}</span>
    </div>
  </div>
</template>
