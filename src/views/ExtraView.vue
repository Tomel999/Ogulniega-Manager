<script setup>
import BackButton from "../components/BackButton.vue";
import NavigationCard from "../components/NavigationCard.vue";
defineEmits(['back']);
</script>

<template>
  <div class="view">
    <BackButton @click="$emit('back')" />
    <h2 class="section-title">{{ $t('extra.title') }}</h2>
    <div class="buttons-grid">
      <NavigationCard :label="$t('extra.shaderpacks')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('extra.resourcepacks')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="20" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
      </NavigationCard>
    </div>
  </div>
</template>
