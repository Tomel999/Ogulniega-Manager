<script setup>
import { ref, onMounted, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import BackButton from '../components/BackButton.vue'
import { useGameVersions } from '../composables/useGameVersions'
import { invoke } from '@tauri-apps/api/core'

defineEmits(['back'])

const { t } = useI18n()
const { getVersions } = useGameVersions()

const PER_PAGE = 16

const state = ref('loading-versions')
const versions = ref([])
const selectedVersion = ref(null)
const mods = ref([])
const error = ref('')
const statusMessage = ref('')
const busy = ref(false)
const query = ref('')
const currentPage = ref(1)

const filteredMods = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return mods.value
  return mods.value.filter(m =>
    m.filename.toLowerCase().includes(q) ||
    (m.sub_path || '').toLowerCase().includes(q)
  )
})

const totalHits = computed(() => filteredMods.value.length)
const totalPages = computed(() => Math.max(1, Math.ceil(totalHits.value / PER_PAGE)))

const paginatedMods = computed(() => {
  const offset = (currentPage.value - 1) * PER_PAGE
  return filteredMods.value.slice(offset, offset + PER_PAGE)
})

const visiblePages = computed(() => {
  const pages = []
  const lastPage = totalPages.value
  const current = currentPage.value

  if (lastPage <= 7) {
    for (let i = 1; i <= lastPage; i += 1) pages.push(i)
    return pages
  }

  pages.push(1)

  const start = Math.max(2, current - 1)
  const end = Math.min(lastPage - 1, current + 1)

  if (start > 2) pages.push('...')

  for (let i = start; i <= end; i += 1) {
    pages.push(i)
  }

  if (end < lastPage - 1) pages.push('...')

  pages.push(lastPage)
  return pages
})

watch(query, () => {
  currentPage.value = 1
})

watch(currentPage, () => {
  if (state.value === 'mods-list') statusMessage.value = ''
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
  state.value = 'loading-mods'
  error.value = ''
  statusMessage.value = ''
  query.value = ''
  currentPage.value = 1
  try {
    mods.value = await invoke('list_mods', { gameVersion: v })
    state.value = 'mods-list'
  } catch (e) {
    error.value = e.message
    state.value = 'error'
  }
}

async function refreshMods() {
  try {
    mods.value = await invoke('list_mods', { gameVersion: selectedVersion.value })
  } catch (e) {
    error.value = e.message
  }
}

function goToPage(n) {
  if (n >= 1 && n <= totalPages.value) currentPage.value = n
}

async function handleModClick(mod) {
  if (busy.value) return

  if (mod.is_disabled) {
    await removeMod(mod)
  } else if (mod.sub_path === 'preinstalled') {
    await disableMod(mod)
  } else {
    await removeMod(mod)
  }
}

async function disableMod(mod) {
  busy.value = true
  error.value = ''
  statusMessage.value = `${mod.filename}...`
  try {
    const newId = await invoke('disable_mod', {
      gameVersion: selectedVersion.value,
      subPath: mod.sub_path || '',
      filename: mod.filename
    })
    statusMessage.value = t('disable.disabled', { filename: mod.filename, id: newId })
    await refreshMods()
  } catch (e) {
    error.value = e.message
    statusMessage.value = ''
  } finally {
    busy.value = false
  }
}

async function removeMod(mod) {
  if (!window.confirm(t('disable.confirmRemove', { filename: mod.filename }))) return
  busy.value = true
  error.value = ''
  statusMessage.value = `${mod.filename}...`
  try {
    await invoke('delete_mod_file', {
      gameVersion: selectedVersion.value,
      subPath: mod.sub_path || '',
      filename: mod.filename
    })
    statusMessage.value = t('disable.removed', { filename: mod.filename })
    await refreshMods()
  } catch (e) {
    error.value = e.message
    statusMessage.value = ''
  } finally {
    busy.value = false
  }
}

function backToVersions() {
  selectedVersion.value = null
  mods.value = []
  statusMessage.value = ''
  error.value = ''
  query.value = ''
  currentPage.value = 1
  state.value = versions.value.length > 0 ? 'select-version' : 'no-versions'
}
</script>

<template>
  <div class="view">
    <BackButton @click="$emit('back')" />

    <h2 class="section-title">{{ $t('disable.title') }}</h2>

    <div v-if="error" class="error-text">{{ error }}</div>

    <div v-if="state === 'loading-versions'" class="status-text">{{ $t('disable.loading') }}</div>

    <div v-if="state === 'no-versions'" class="status-text">{{ $t('disable.noVersions') }}</div>

    <div v-if="state === 'select-version'" class="version-picker">
      <h3 class="picker-title">{{ $t('disable.selectVersion') }}</h3>
      <div class="buttons-grid">
        <div
          v-for="v in versions"
          :key="v"
          class="button-card version-card"
          @click="selectVersion(v)"
        >
          <div class="icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>
          </div>
          <span class="button-label">{{ v }}</span>
        </div>
      </div>
    </div>

    <div v-if="state === 'loading-mods'" class="status-text">{{ $t('disable.loadingMods') }}</div>

    <div v-if="state === 'mods-list'" class="mods-section">
      <h3 class="picker-title">{{ selectedVersion }}</h3>

      <div class="search-row">
        <input
          v-model="query"
          class="search-input"
          :placeholder="$t('disable.searchPlaceholder')"
        />
      </div>

      <p class="picker-hint">{{ $t('disable.clickHint') }}</p>
      <div v-if="statusMessage" class="status-text success-text">{{ statusMessage }}</div>

      <div v-if="filteredMods.length === 0" class="status-text">
        {{ query ? $t('disable.noResults') : $t('disable.noMods') }}
      </div>

      <div v-else class="mods-list">
        <div
          v-for="mod in paginatedMods"
          :key="`${mod.sub_path}/${mod.filename}`"
          :class="['mod-row', { 'is-disabled': mod.is_disabled, 'is-busy': busy }]"
          @click="handleModClick(mod)"
        >
          <div class="mod-row-content">
            <span v-if="mod.sub_path" class="mod-subpath">{{ mod.sub_path }}/</span>
            <span class="mod-filename">{{ mod.filename }}</span>
          </div>
          <span :class="['mod-status', { disabled: mod.is_disabled }]">
            {{ mod.is_disabled ? $t('disable.disabledLabel') : $t('disable.activeLabel') }}
          </span>
        </div>
      </div>

      <div v-if="totalPages > 1" class="pagination">
        <button class="page-btn" :disabled="currentPage <= 1" @click="goToPage(currentPage - 1)">‹</button>
        <button
          v-for="page in visiblePages"
          :key="`${page}-${typeof page}`"
          :class="['page-btn', { active: page === currentPage, ellipsis: page === '...' }]"
          :disabled="page === '...'"
          @click="page !== '...' && goToPage(page)"
        >{{ page }}</button>
        <button class="page-btn" :disabled="currentPage >= totalPages" @click="goToPage(currentPage + 1)">›</button>
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

.mods-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.search-row {
  width: 100%;
  max-width: 600px;
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

.mods-list {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 640px;
  gap: 0.5rem;
  margin: 0.8rem 0;
}

.mod-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  padding: 0.7rem 1rem;
  background: rgba(255, 255, 255, 0.06);
  border: 2px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.mod-row:hover {
  background: rgba(255, 215, 0, 0.1);
  border-color: rgba(255, 215, 0, 0.4);
}

.mod-row.is-disabled {
  opacity: 0.55;
  cursor: not-allowed;
  background: rgba(255, 255, 255, 0.03);
  border-color: rgba(255, 255, 255, 0.08);
}

.mod-row.is-disabled:hover {
  background: rgba(255, 255, 255, 0.03);
  border-color: rgba(255, 255, 255, 0.08);
}

.mod-row.is-busy {
  pointer-events: none;
  opacity: 0.7;
}

.mod-row-content {
  display: flex;
  align-items: baseline;
  gap: 0.3rem;
  flex: 1;
  min-width: 0;
}

.mod-subpath {
  font-family: 'Inter', sans-serif;
  font-size: 0.8rem;
  color: rgba(255, 215, 0, 0.7);
  white-space: nowrap;
  flex-shrink: 0;
}

.mod-filename {
  font-family: 'Inter', sans-serif;
  font-size: 0.95rem;
  color: var(--text-color);
  word-break: break-all;
  text-align: left;
}

.mod-status {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.85rem;
  color: #51cf66;
  padding: 0.25rem 0.6rem;
  border-radius: 6px;
  background: rgba(81, 207, 102, 0.12);
  flex-shrink: 0;
}

.mod-status.disabled {
  color: #ff8787;
  background: rgba(255, 135, 135, 0.12);
}

.pagination {
  display: flex;
  gap: 0.4rem;
  margin-top: 0.5rem;
  margin-bottom: 0.5rem;
}

.page-btn {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  border: 2px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-color);
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
}

.page-btn.active {
  border-color: #ffd700;
  background: rgba(255, 215, 0, 0.2);
  color: #ffd700;
}

.page-btn.ellipsis {
  opacity: 0.65;
  cursor: default;
}

.page-btn:disabled {
  opacity: 0.3;
  cursor: default;
}

.page-btn:hover:not(:disabled):not(.active) {
  border-color: rgba(255, 255, 255, 0.3);
}

.cancel-btn {
  margin-top: 0.5rem;
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
</style>
