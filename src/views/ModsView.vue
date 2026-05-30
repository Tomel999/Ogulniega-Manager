<script setup>
import BackButton from "../components/BackButton.vue";
import NavigationCard from "../components/NavigationCard.vue";
defineEmits(['back', 'navigate']);
</script>

<template>
  <div class="view">
    <BackButton @click="$emit('back')" />
    <h2 class="section-title">{{ $t('mods.title') }}</h2>
    <div class="buttons-grid">
      <NavigationCard :label="$t('mods.download')" @click="$emit('navigate', 'download')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('mods.disable')" :disabled="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('mods.compatibility')" :disabled="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('mods.duplicates')" :disabled="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
      </NavigationCard>
    </div>
  </div>
</template>
