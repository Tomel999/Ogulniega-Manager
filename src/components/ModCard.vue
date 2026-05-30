<script setup>
defineProps({
  name: { type: String, required: true },
  iconUrl: { type: String, default: '' }
})
defineEmits(['click'])
</script>

<template>
  <div class="mod-card" @click="$emit('click')">
    <div class="icon">
      <img v-if="iconUrl" :src="iconUrl" alt="" class="mod-icon" />
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>
    </div>
    <span class="button-label">{{ name }}</span>
  </div>
</template>

<style scoped>
.mod-card {
  background-color: var(--card-bg, rgba(255, 255, 255, 0.05));
  border-radius: 12px;
  padding: 1.2rem 0.5rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 120px;
  height: 120px;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.mod-card:hover {
  background-color: var(--card-hover, rgba(255, 255, 255, 0.1));
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0,0,0,0.15);
}

.mod-card:active {
  transform: translateY(0);
}

.icon {
  width: 48px;
  height: 48px;
  margin-bottom: 0.8rem;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.2);
}

.mod-icon {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.icon svg {
  width: 24px;
  height: 24px;
  color: rgba(255, 255, 255, 0.5);
}

.button-label {
  font-weight: 600;
  font-size: 0.95rem;
  font-family: 'Pixelify Sans', sans-serif;
  letter-spacing: 0.5px;
  color: #fff;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100px;
  white-space: nowrap;
}
</style>
