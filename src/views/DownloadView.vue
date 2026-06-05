<script setup>
import { ref, watch, computed } from 'vue'
import BackButton from '../components/BackButton.vue'
import ModCard from '../components/ModCard.vue'
import { useModrinthApi } from '../composables/useModrinthApi'
import { useCurseForgeApi } from '../composables/useCurseForgeApi'
import { useGameVersions } from '../composables/useGameVersions'
import { fetch } from '@tauri-apps/plugin-http'
import { invoke } from '@tauri-apps/api/core'
defineEmits(['back'])

const modrinthApi = useModrinthApi()
const { searchMods, getProjectVersions, getProject } = modrinthApi
const curseForge = useCurseForgeApi()
const { getVersions } = useGameVersions()

const PER_PAGE = 16

const query = ref('')
const source = ref('modrinth')
const state = ref('idle')
const mods = ref([])
const selectedMod = ref(null)
const versions = ref([])
const error = ref('')
const statusMessage = ref('')
const currentPage = ref(1)
const totalHits = ref(0)

const totalPages = computed(() => Math.max(1, Math.ceil(totalHits.value / PER_PAGE)))
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

let searchTimeout = null

watch(query, () => {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => { currentPage.value = 1; doSearch() }, 300)
})

watch(currentPage, () => {
  if (query.value.trim()) doSearch()
})

async function doSearch() {
  const q = query.value.trim()
  selectedMod.value = null
  versions.value = []
  statusMessage.value = ''

  if (!q) {
    mods.value = []
    totalHits.value = 0
    state.value = 'idle'
    return
  }

  state.value = 'searching'
  error.value = ''
  const offset = (currentPage.value - 1) * PER_PAGE

  try {
    if (source.value === 'modrinth') {
      const result = await searchMods(q, '', offset, PER_PAGE)
      mods.value = result.hits || []
      totalHits.value = result.total_hits || 0
    } else {
      if (!(await curseForge.hasApiKey())) {
        error.value = 'CurseForge requires an API key. Set it in Settings.'
        mods.value = []
        state.value = 'error'
        return
      }
      const result = await curseForge.searchMods(q, '', offset, PER_PAGE)
      mods.value = (result.data || []).map(m => ({
        project_id: m.id,
        slug: m.slug || m.name,
        title: m.name,
        icon_url: m.logo ? m.logo.url : '',
        latestFilesIndexes: m.latestFilesIndexes || [],
      }))
      totalHits.value = result.pagination?.totalCount || mods.value.length
    }
    state.value = mods.value.length > 0 ? 'results' : 'idle'
  } catch (e) {
    error.value = e.message
    state.value = 'error'
  }
}

function goToPage(n) {
  if (n >= 1 && n <= totalPages.value) currentPage.value = n
}

async function selectMod(mod) {
  selectedMod.value = mod
  state.value = 'selecting-version'
  statusMessage.value = ''

  try {
    const allVersions = await getVersions()
    
    let supportedBaseVersions = []
    if (source.value === 'modrinth') {
      supportedBaseVersions = mod.versions || []
    } else {
      supportedBaseVersions = (mod.latestFilesIndexes || []).map(idx => idx.gameVersion)
    }

    versions.value = allVersions.filter(v => {
      const baseVersion = v.split('-')[0]
      return supportedBaseVersions.includes(baseVersion)
    })

    if (allVersions.length > 0 && versions.value.length === 0) {
      error.value = 'This mod does not support any of your game versions.'
      state.value = 'error'
    } else if (versions.value.length === 0) {
      error.value = 'No game versions found in mods folder.'
      state.value = 'error'
    }
  } catch (e) {
    error.value = e.message
    state.value = 'error'
  }
}

async function downloadSingleFile(downloadUrl, filename, gameVersion) {
  const res = await fetch(downloadUrl)
  if (!res.ok) throw new Error(`Download failed: ${res.status}`)
  const blob = await res.blob()
  const arrayBuffer = await blob.arrayBuffer()
  const data = new Uint8Array(arrayBuffer)
  await invoke('write_mod_file', { gameVersion, filename, data })
}

async function getExistingFilenames(gameVersion) {
  try {
    const gameFiles = await invoke('list_directory_files', { dirName: gameVersion })
    const preinstalledFiles = await invoke('list_directory_files', { dirName: `${gameVersion}/preinstalled` })
    const sharedPreinstalledFiles = await invoke('list_directory_files', { dirName: 'preinstalled' })
    return [...gameFiles, ...preinstalledFiles, ...sharedPreinstalledFiles]
  } catch {
    return []
  }
}

function normalizeIdentifier(value) {
  return (value || '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function buildIdentifiers(...values) {
  const identifiers = new Set()

  for (const value of values.flat()) {
    const raw = (value ?? '').toString().trim()
    if (!raw) continue

    identifiers.add(raw.toLowerCase())
    identifiers.add(normalizeIdentifier(raw))

    const withoutExt = raw.replace(/\.[^.]+$/, '')
    identifiers.add(withoutExt.toLowerCase())
    identifiers.add(normalizeIdentifier(withoutExt))
  }

  return [...identifiers].filter(Boolean)
}

function isInstalledByMetadata(existingFiles, metadata = {}, fileName = '') {
  const identifiers = buildIdentifiers(
    fileName,
    metadata.slug,
    metadata.title,
    metadata.name,
    metadata.project_id,
    metadata.id
  )

  return existingFiles.some(file => {
    const normalizedFile = file.toLowerCase()
    const normalizedStem = normalizedFile.replace(/\.[^.]+$/, '')
    return identifiers.some(identifier => {
      const normalizedIdentifier = normalizeIdentifier(identifier)
      return (
        normalizedFile === identifier ||
        normalizedFile.includes(identifier) ||
        normalizedStem === identifier ||
        normalizedStem.includes(identifier) ||
        normalizedFile.includes(normalizedIdentifier) ||
        normalizedStem.includes(normalizedIdentifier)
      )
    })
  })
}

async function downloadDependencies(projectIds, baseVersion, gameVersion, existingFiles) {
  const results = []

  for (const projectId of projectIds) {
    try {
      if (source.value === 'modrinth') {
        const project = await getProject(projectId)
        const versions = await getProjectVersions(project.slug, baseVersion)
        if (versions.length === 0) continue
        const v = versions[0]
        const file = v.files.find(f => f.primary) || v.files[0]
        if (!file || !file.url) continue

        if (isInstalledByMetadata(existingFiles, project, file.filename)) {
          results.push({ success: true, skipped: true, name: project.title })
          continue
        }

        await downloadSingleFile(file.url, file.filename, gameVersion)
        existingFiles.push(file.filename)
        results.push({ success: true, name: project.title })
      } else {
        const modData = await curseForge.getMod(projectId)
        const file = (modData.latestFiles || []).find(f =>
          f.gameVersions && f.gameVersions.includes(baseVersion) &&
          f.gameVersions.some(v => v.toLowerCase() === 'fabric')
        ) || (modData.latestFiles || [])[0]
        if (!file || !file.downloadUrl) continue

        if (isInstalledByMetadata(existingFiles, modData, file.fileName)) {
          results.push({ success: true, skipped: true, name: modData.name })
          continue
        }

        await downloadSingleFile(file.downloadUrl, file.fileName, gameVersion)
        existingFiles.push(file.fileName)
        results.push({ success: true, name: modData.name })
      }
    } catch (e) {
      results.push({ success: false })
    }
  }
  return results
}

async function downloadMod(gameVersion) {
  state.value = 'downloading'
  statusMessage.value = 'Downloading...'
  error.value = ''

  try {
    let downloadUrl, filename
    const baseVersion = gameVersion.split('-')[0]
    let dependencyProjects = []
    const existingFiles = await getExistingFilenames(gameVersion)

    if (source.value === 'modrinth') {
      const versionData = await getProjectVersions(selectedMod.value.slug, baseVersion)
      if (versionData.length === 0) {
        error.value = `No ${baseVersion} version found for this mod.`
        state.value = 'error'
        return
      }
      const v = versionData[0]
      const file = v.files.find(f => f.primary) || v.files[0]
      downloadUrl = file.url
      filename = file.filename

      if (isInstalledByMetadata(existingFiles, selectedMod.value, filename)) {
        statusMessage.value = `Already installed in ${gameVersion}/${filename}`
        state.value = 'done'
        return
      }

      if (v.dependencies) {
        dependencyProjects = v.dependencies
          .filter(d => d.dependency_type === 'required' && d.project_id)
          .map(d => d.project_id)
      }
    } else {
      const modData = await curseForge.getMod(selectedMod.value.project_id)
      const files = await curseForge.getModFiles(selectedMod.value.project_id, baseVersion)
      if (!files || files.length === 0) {
        error.value = `No ${baseVersion} file found for this mod.`
        state.value = 'error'
        return
      }
      const file = files.find(f =>
        f.gameVersions && f.gameVersions.includes(baseVersion) &&
        f.gameVersions.some(v => v.toLowerCase() === 'fabric')
      ) || files[0]
      downloadUrl = file.downloadUrl
      filename = file.fileName

      if (isInstalledByMetadata(existingFiles, modData, filename)) {
        statusMessage.value = `Already installed in ${gameVersion}/${filename}`
        state.value = 'done'
        return
      }

      if (file.dependencies) {
        dependencyProjects = file.dependencies
          .filter(d => d.relationType === 1)
          .map(d => d.modId)
      }
    }

    await downloadSingleFile(downloadUrl, filename, gameVersion)

    if (dependencyProjects.length > 0) {
      statusMessage.value = 'Downloading dependencies...'
      const depResults = await downloadDependencies(dependencyProjects, baseVersion, gameVersion, existingFiles)
      const downloaded = depResults.filter(r => r.success && !r.skipped).map(r => r.name).filter(Boolean)
      const skipped = depResults.filter(r => r.skipped).map(r => r.name).filter(Boolean)
      const failedCount = depResults.filter(r => !r.success).length
      let summary = ''
      if (downloaded.length > 0) summary += ` with ${downloaded.join(', ')}`
      if (skipped.length > 0) summary += `${downloaded.length > 0 ? ' ' : ' '}(${skipped.length} already installed)`
      const failMsg = failedCount > 0 ? `, ${failedCount} dependenc${failedCount === 1 ? 'y' : 'ies'} failed` : ''
      statusMessage.value = `Downloaded to ${gameVersion}/${filename}${summary}${failMsg}`
    } else {
      statusMessage.value = `Downloaded to ${gameVersion}/${filename}`
    }

    state.value = 'done'
  } catch (e) {
    error.value = e.message
    state.value = 'error'
  }
}

function cancelSelection() {
  selectedMod.value = null
  versions.value = []
  currentPage.value = 1
  totalHits.value = 0
  statusMessage.value = ''
  state.value = mods.value.length > 0 ? 'results' : 'idle'
}

function resetSearch() {
  selectedMod.value = null
  versions.value = []
  currentPage.value = 1
  totalHits.value = 0
  state.value = 'idle'
  query.value = ''
  mods.value = []
  error.value = ''
  statusMessage.value = ''
}
</script>

<template>
  <div class="view">
    <BackButton v-if="state !== 'selecting-version'" @click="$emit('back')" />

    <h2 class="section-title" v-show="state === 'idle'">{{ $t('download.title') }}</h2>

    <div class="search-row">
      <input
        v-model="query"
        class="search-input"
        :placeholder="$t('download.searchPlaceholder')"
      />
    </div>

    <div class="source-toggle">
      <button
        :class="['source-btn', { active: source === 'modrinth' }]"
        @click="source = 'modrinth'; currentPage = 1; selectedMod = null; versions = []; doSearch()"
      >
        Modrinth
      </button>
      <button
        :class="['source-btn', { active: source === 'curseforge' }]"
        @click="source = 'curseforge'; currentPage = 1; selectedMod = null; versions = []; doSearch()"
      >
        CurseForge
      </button>
    </div>

    <div v-if="state === 'searching'" class="status-text">{{ $t('download.searching') }}</div>

    <div v-if="error" class="error-text">{{ error }}</div>

    <div v-if="state === 'results' && !selectedMod" class="mods-section">
      <div class="mods-grid">
        <ModCard
          v-for="m in mods"
          :key="m.project_id"
          :name="m.title"
          :icon-url="m.icon_url"
          @click="selectMod(m)"
        />
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
    </div>

    <div v-if="state === 'selecting-version' && selectedMod" class="version-picker">
      <h3 class="picker-title">{{ $t('download.selectVersion') }}</h3>
      <p class="picker-mod-name">{{ selectedMod.title }}</p>
      <div class="buttons-grid">
        <div
          v-for="v in versions"
          :key="v"
          class="button-card version-card"
          @click="downloadMod(v)"
        >
          <div class="icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
          </div>
          <span class="button-label">{{ v }}</span>
        </div>
      </div>
      <button class="cancel-btn" @click="cancelSelection">{{ $t('common.back') }}</button>
    </div>

    <div v-if="state === 'downloading'" class="status-text">{{ statusMessage }}</div>

    <div v-if="state === 'done'" class="status-text success-text">{{ statusMessage }}</div>

    <button v-if="state === 'done'" class="continue-btn" @click="resetSearch">{{ $t('download.continue') }}</button>
  </div>
</template>

<style scoped>
.search-row {
  width: 100%;
  max-width: 600px;
  margin-bottom: 0.5rem;
}

.search-input {
  width: 100%;
  padding: 1.2rem 1.4rem;
  background-color: rgba(255, 255, 255, 0.08);
  border: 2px solid rgba(255, 255, 255, 0.15);
  border-radius: 14px;
  color: var(--text-color);
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.3rem;
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

.source-toggle {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.source-btn {
  padding: 0.5rem 1.2rem;
  border-radius: 8px;
  border: 2px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-color);
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.source-btn.active {
  border-color: #ffd700;
  background: rgba(255, 215, 0, 0.15);
}

.source-btn:hover {
  border-color: rgba(255, 255, 255, 0.3);
}

.buttons-grid {
  display: flex;
  justify-content: center;
  gap: 1.5rem;
  margin-bottom: 1.5rem;
  width: 100%;
  flex-wrap: wrap;
}

.mods-grid {
  display: flex;
  justify-content: center;
  gap: 1.5rem;
  margin-bottom: 1.5rem;
  width: 100%;
  flex-wrap: wrap;
  max-width: 800px;
}

.mods-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.pagination {
  display: flex;
  gap: 0.4rem;
  margin-top: 0.5rem;
  margin-bottom: 1rem;
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
  color: #d8c8b8;
}

.picker-mod-name {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.1rem;
  margin: 0 0 1.5rem 0;
  color: #ffd700;
}

.version-card {
  width: 100px !important;
  height: 100px !important;
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

.continue-btn {
  margin-top: 1rem;
  padding: 0.7rem 2rem;
  border-radius: 8px;
  border: 2px solid #ffd700;
  background: rgba(255, 215, 0, 0.15);
  color: #ffd700;
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.1rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.continue-btn:hover {
  background: rgba(255, 215, 0, 0.25);
}

.status-text {
  font-family: 'Inter', sans-serif;
  font-size: 1rem;
  color: #d8c8b8;
  margin: 1rem 0;
}

.error-text {
  font-family: 'Inter', sans-serif;
  font-size: 0.95rem;
  color: #ff6b6b;
  margin: 1rem 0;
  text-align: center;
  max-width: 400px;
}

.success-text {
  color: #51cf66;
}
</style>
