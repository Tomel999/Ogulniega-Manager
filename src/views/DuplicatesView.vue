<script setup>
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BackButton from '../components/BackButton.vue'
import { useGameVersions } from '../composables/useGameVersions'
import { invoke } from '@tauri-apps/api/core'

defineEmits(['back'])

const { t } = useI18n()
const { getVersions } = useGameVersions()

const state = ref('loading-versions')
const versions = ref([])
const selectedVersion = ref(null)
const duplicates = ref([])
const error = ref('')
const statusMessage = ref('')
const busy = ref(false)
const query = ref('')

const filteredDuplicates = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return duplicates.value
  return duplicates.value.filter(d => {
    if (d.mod_id.toLowerCase().includes(q)) return true
    return [...d.preinstalled, ...d.regular].some(m =>
      m.filename.toLowerCase().includes(q)
    )
  })
})

onMounted(loadVersions)

async function loadVersions() {
  state.value = 'loading-versions'
  error.value = ''
  try {
    versions.value = await getVersions()
    state.value = versions.value.length > 0 ? 'select-version' : 'no-versions'
  } catch (e) {
    error.value = e.message
    state.value = 'error'
  }
}

async function selectVersion(v) {
  selectedVersion.value = v
  state.value = 'loading-duplicates'
  error.value = ''
  statusMessage.value = ''
  query.value = ''
  try {
    duplicates.value = await invoke('find_duplicate_mods', { gameVersion: v })
    state.value = duplicates.value.length > 0 ? 'duplicates-list' : 'no-duplicates'
  } catch (e) {
    error.value = e.message
    state.value = 'error'
  }
}

async function refreshDuplicates() {
  try {
    duplicates.value = await invoke('find_duplicate_mods', { gameVersion: selectedVersion.value })
    if (duplicates.value.length === 0) {
      state.value = 'no-duplicates'
    }
  } catch (e) {
    error.value = e.message
  }
}

async function removeMod(mod) {
  if (busy.value) return
  if (!window.confirm(t('duplicates.confirmRemove', { filename: mod.filename }))) return
  busy.value = true
  error.value = ''
  statusMessage.value = `${mod.filename}...`
  try {
    await invoke('delete_mod_file', {
      gameVersion: selectedVersion.value,
      subPath: mod.sub_path || '',
      filename: mod.filename
    })
    statusMessage.value = t('duplicates.removed', { filename: mod.filename })
    await refreshDuplicates()
  } catch (e) {
    error.value = e.message
    statusMessage.value = ''
  } finally {
    busy.value = false
  }
}

function backToVersions() {
  selectedVersion.value = null
  duplicates.value = []
  statusMessage.value = ''
  error.value = ''
  query.value = ''
  state.value = versions.value.length > 0 ? 'select-version' : 'no-versions'
}
</script>

<template>
  <div class="view">
    <BackButton @click="$emit('back')" />

    <h2 class="section-title">{{ $t('duplicates.title') }}</h2>

    <div v-if="error" class="error-text">{{ error }}</div>

    <div v-if="state === 'loading-versions'" class="status-text">{{ $t('duplicates.loading') }}</div>
    <div v-if="state === 'no-versions'" class="status-text">{{ $t('duplicates.noVersions') }}</div>

    <div v-if="state === 'select-version'" class="version-picker">
      <h3 class="picker-title">{{ $t('duplicates.selectVersion') }}</h3>
      <div class="buttons-grid">
        <div
          v-for="v in versions"
          :key="v"
          class="button-card version-card"
          @click="selectVersion(v)"
        >
          <div class="icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          </div>
          <span class="button-label">{{ v }}</span>
        </div>
      </div>
    </div>

    <div v-if="state === 'loading-duplicates'" class="status-text">{{ $t('duplicates.loadingDuplicates') }}</div>

    <div v-if="state === 'no-duplicates'" class="status-text success-text">
      {{ $t('duplicates.noDuplicates', { version: selectedVersion }) }}
      <div>
        <button class="cancel-btn" @click="backToVersions">{{ $t('common.back') }}</button>
      </div>
    </div>

    <div v-if="state === 'duplicates-list'" class="duplicates-section">
      <h3 class="picker-title">{{ selectedVersion }}</h3>
      <p class="picker-hint">{{ $t('duplicates.hint') }}</p>

      <div class="search-row">
        <input
          v-model="query"
          class="search-input"
          :placeholder="$t('duplicates.searchPlaceholder')"
        />
      </div>

      <div v-if="statusMessage" class="status-text success-text">{{ statusMessage }}</div>

      <div v-if="filteredDuplicates.length === 0" class="status-text">
        {{ $t('duplicates.noResults') }}
      </div>

      <div v-else class="duplicates-list">
        <div
          v-for="dup in filteredDuplicates"
          :key="dup.mod_id"
          class="duplicate-group"
        >
          <div class="duplicate-header">
            <span class="duplicate-icon">⚠</span>
            <span class="duplicate-id">{{ dup.mod_id }}</span>
            <span class="duplicate-count">
              {{ $t('duplicates.countLabel', { count: dup.regular.length + dup.preinstalled.length }) }}
            </span>
          </div>
          <div class="duplicate-columns">
            <div v-if="dup.preinstalled.length > 0" class="duplicate-column">
              <h4 class="column-title">{{ $t('duplicates.preinstalled') }}</h4>
              <div class="file-list">
                <div
                  v-for="mod in dup.preinstalled"
                  :key="`pre-${mod.filename}`"
                  :class="['file-row', 'file-removable', { 'is-busy': busy }]"
                  @click="removeMod(mod)"
                >
                  <span class="file-name">{{ mod.filename }}</span>
                  <span class="file-action">{{ $t('duplicates.removeAction') }}</span>
                </div>
              </div>
            </div>
            <div v-if="dup.regular.length > 0" class="duplicate-column">
              <h4 class="column-title">{{ $t('duplicates.regular') }}</h4>
              <div class="file-list">
                <div
                  v-for="mod in dup.regular"
                  :key="`reg-${mod.filename}`"
                  :class="['file-row', 'file-removable', { 'is-busy': busy }]"
                  @click="removeMod(mod)"
                >
                  <span class="file-name">{{ mod.filename }}</span>
                  <span class="file-action">{{ $t('duplicates.removeAction') }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <button class="cancel-btn" @click="backToVersions">{{ $t('common.back') }}</button>
    </div>
  </div>
</template>

<style scoped>
.version-picker {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.picker-title {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.3rem;
  margin: 0 0 0.3rem 0;
  color: #ffd700;
}

.picker-hint {
  font-family: 'Inter', sans-serif;
  font-size: 0.95rem;
  margin: 0 0 1.2rem 0;
  color: #d8c8b8;
  text-align: center;
  max-width: 600px;
}

.buttons-grid {
  display: flex;
  justify-content: center;
  gap: 1.5rem;
  margin-bottom: 1.5rem;
  width: 100%;
  flex-wrap: wrap;
}

.version-card {
  width: 100px !important;
  height: 100px !important;
}

.duplicates-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.search-row {
  width: 100%;
  max-width: 700px;
  margin-bottom: 0.5rem;
}

.search-input {
  width: 100%;
  padding: 0.9rem 1.2rem;
  background-color: rgba(255, 255, 255, 0.08);
  border: 2px solid rgba(255, 255, 255, 0.15);
  border-radius: 12px;
  color: var(--text-color);
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.1rem;
  letter-spacing: 0.5px;
  box-sizing: border-box;
  transition: border-color 0.2s ease;
}

.search-input::placeholder {
  color: rgba(255, 255, 255, 0.4);
}

.search-input:focus {
  outline: none;
  border-color: #ffd700;
}

.duplicates-list {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 760px;
  gap: 1rem;
  margin: 0.8rem 0;
}

.duplicate-group {
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.04);
  border: 2px solid rgba(255, 200, 87, 0.35);
  border-radius: 12px;
  padding: 0.9rem 1rem 1rem 1rem;
}

.duplicate-header {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding-bottom: 0.6rem;
  border-bottom: 1px dashed rgba(255, 200, 87, 0.25);
  margin-bottom: 0.7rem;
}

.duplicate-icon {
  font-size: 1.1rem;
  color: #ffc857;
}

.duplicate-id {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.1rem;
  color: #ffd700;
  flex: 1;
  word-break: break-all;
}

.duplicate-count {
  font-family: 'Inter', sans-serif;
  font-size: 0.8rem;
  color: #d8c8b8;
  background: rgba(255, 255, 255, 0.06);
  padding: 0.2rem 0.55rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.duplicate-columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.8rem;
}

.duplicate-column {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.column-title {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.95rem;
  margin: 0;
  color: #d8c8b8;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.file-list {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.file-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
  padding: 0.55rem 0.75rem;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.05);
  border: 2px solid rgba(255, 255, 255, 0.1);
}

.file-name {
  font-family: 'Inter', sans-serif;
  font-size: 0.85rem;
  color: var(--text-color);
  word-break: break-all;
  text-align: left;
}

.file-removable {
  cursor: pointer;
  transition: all 0.15s ease;
  border-color: rgba(255, 107, 107, 0.25);
}

.file-removable:hover {
  background: rgba(255, 107, 107, 0.12);
  border-color: rgba(255, 107, 107, 0.55);
}

.file-removable.is-busy {
  pointer-events: none;
  opacity: 0.6;
}

.file-action {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.78rem;
  color: #ff8787;
  background: rgba(255, 135, 135, 0.12);
  padding: 0.2rem 0.55rem;
  border-radius: 5px;
  flex-shrink: 0;
}

.cancel-btn {
  margin-top: 1rem;
  padding: 0.6rem 1.5rem;
  border-radius: 8px;
  border: 2px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-color);
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.cancel-btn:hover {
  border-color: rgba(255, 255, 255, 0.3);
}

.status-text {
  font-family: 'Inter', sans-serif;
  font-size: 1rem;
  color: #d8c8b8;
  margin: 0.5rem 0;
  text-align: center;
}

.success-text {
  color: #51cf66;
}

.error-text {
  font-family: 'Inter', sans-serif;
  font-size: 0.95rem;
  color: #ff6b6b;
  margin: 1rem 0;
  text-align: center;
  max-width: 500px;
  word-break: break-word;
}

@media (max-width: 600px) {
  .duplicate-columns {
    grid-template-columns: 1fr;
  }
}
</style>
