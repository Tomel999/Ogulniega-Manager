<script setup>
import { useI18n } from "vue-i18n";
import BackButton from "../components/BackButton.vue";
import { SUPPORTED_LOCALES } from "../i18n";
defineEmits(['back']);

const { locale } = useI18n();

async function setLanguage(value) {
  locale.value = value;
  localStorage.setItem("locale", value);
}

const savedLocale = localStorage.getItem("locale");
if (savedLocale && SUPPORTED_LOCALES.some((l) => l.value === savedLocale)) {
  locale.value = savedLocale;
}
</script>

<template>
  <div class="view view--full">
    <BackButton @click="$emit('back')" />
    <h2 class="section-title">{{ $t('settings.title') }}</h2>
    <div class="settings-body">
      <div class="settings-card">
        <label class="settings-section-title">{{ $t('settings.language') }}</label>
        <div class="select-wrapper">
          <select class="settings-select" :value="locale" @change="setLanguage($event.target.value)">
            <option v-for="lang in SUPPORTED_LOCALES" :key="lang.value" :value="lang.value">
              {{ lang.label }}
            </option>
          </select>
          <span class="select-arrow">&#9660;</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  flex: 1;
  padding: 1rem 0;
}

.settings-card {
  background-color: var(--card-bg);
  border-radius: 16px;
  padding: 2rem 2.5rem;
  width: 100%;
  max-width: 400px;
  box-shadow: 0 4px 6px rgba(0,0,0,0.1);
}

.settings-section-title {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.3rem;
  letter-spacing: 0.5px;
  display: block;
  margin-bottom: 1.2rem;
  text-align: center;
  color: #d8c8b8;
}

.select-wrapper {
  position: relative;
  width: 100%;
}

.settings-select {
  width: 100%;
  padding: 0.9rem 1rem;
  background-color: rgba(255, 255, 255, 0.08);
  border: 2px solid rgba(255, 255, 255, 0.15);
  border-radius: 12px;
  color: var(--text-color);
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.1rem;
  letter-spacing: 0.5px;
  cursor: pointer;
  transition: border-color 0.2s ease;
  -webkit-appearance: none;
  -moz-appearance: none;
  appearance: none;
}

.settings-select:hover {
  border-color: rgba(255, 255, 255, 0.3);
}

.settings-select:focus {
  outline: none;
  border-color: #ffd700;
}

.select-arrow {
  position: absolute;
  right: 1rem;
  top: 50%;
  transform: translateY(-50%);
  color: #d8c8b8;
  font-size: 0.8rem;
  pointer-events: none;
}
</style>
