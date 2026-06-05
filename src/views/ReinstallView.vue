<script setup>
import BackButton from "../components/BackButton.vue";
import NavigationCard from "../components/NavigationCard.vue";
defineEmits(['back', 'navigate']);
</script>

<template>
  <div class="view">
    <BackButton @click="$emit('back')" />
    <h2 class="section-title">{{ $t('launcher.reinstall') }}</h2>
    <div class="buttons-grid">
      <NavigationCard :label="$t('launcher.reinstallAll')" :disabled="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('launcher.reinstallLibs')" :disabled="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="9" y1="21" x2="9" y2="9"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('launcher.reinstallAssets')" :disabled="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="20" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
      </NavigationCard>
      <NavigationCard :label="$t('launcher.reinstallMods')" @click="$emit('navigate', 'reinstallMods')">
        <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><image href="https://avatars.githubusercontent.com/u/21025855?v=4" width="24" height="24"/></svg>
      </NavigationCard>
    </div>
  </div>
</template>
