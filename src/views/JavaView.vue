<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useJavaManager } from '../composables/useJavaManager'
import BackButton from '../components/BackButton.vue'
import NavigationCard from '../components/NavigationCard.vue'

defineEmits(['back'])

const java = useJavaManager()

const state = ref('loading')
const error = ref('')
const platform = ref(null)
const javaPath = ref('')
const distributions = ref([])
const launcherProfiles = ref([])

const selectedDistribution = ref('temurin')
const selectedProfile = ref(null)

const releaseInfo = ref(null)
const releaseLoading = ref(false)
const releaseError = ref('')

const installing = ref(false)
const installProgress = ref({ received: 0, total: 0, phase: '' })
const lastResult = ref(null)
let unlistenProgress = null
let speedWindow = []

const selectedDistro = computed(() => {
  return distributions.value.find((d) => d.id === selectedDistribution.value)
})

const compatibleProfiles = computed(() => {
  if (!selectedDistro.value) return launcherProfiles.value
  return launcherProfiles.value.filter((p) =>
    selectedDistro.value.kinds.map((k) => k.toLowerCase()).includes(p.kind.toLowerCase()),
  )
})

function formatBytes(n) {
  if (!n || n <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let v = n
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i += 1
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`
}

const progressPct = computed(() => {
  const p = installProgress.value
  if (!p || !p.total) return 0
  return Math.min(100, Math.round((p.received / p.total) * 100))
})

const speedBps = computed(() => {
  if (installProgress.value.phase !== 'downloading') return 0
  if (speedWindow.length < 2) return 0
  const oldest = speedWindow[0]
  const newest = speedWindow[speedWindow.length - 1]
  const dt = (newest.t - oldest.t) / 1000
  if (dt <= 0) return 0
  return (newest.r - oldest.r) / dt
})

const etaSeconds = computed(() => {
  if (!speedBps.value || !installProgress.value.total) return 0
  return (installProgress.value.total - installProgress.value.received) / speedBps.value
})

function formatEta(secs) {
  if (!secs || secs <= 0 || !Number.isFinite(secs)) return ''
  if (secs < 60) return `${Math.round(secs)}s`
  const m = Math.floor(secs / 60)
  const s = Math.round(secs % 60)
  if (m < 60) return `${m}m ${s}s`
  const h = Math.floor(m / 60)
  return `${h}h ${m % 60}m`
}

async function loadAll() {
  state.value = 'loading'
  error.value = ''
  try {
    platform.value = await java.getPlatform()
    javaPath.value = await java.getJavaPath()
    distributions.value = await java.listDistributions()
    try {
      const launcher = await java.getLauncherProfiles()
      launcherProfiles.value = launcher.profiles
    } catch (e) {
      error.value = `Could not read launcher.json: ${e.message || e}`
      launcherProfiles.value = []
    }
    pickDefaultProfile()
    state.value = 'ready'
    if (selectedProfile.value) {
      await loadRelease()
    }
  } catch (e) {
    error.value = e.message || String(e)
    state.value = 'error'
  }
}

function pickDefaultProfile() {
  const list = compatibleProfiles.value
  if (list.length === 0) {
    selectedProfile.value = null
    return
  }
  if (
    !selectedProfile.value ||
    !list.find((p) => p.java_name === selectedProfile.value.java_name)
  ) {
    selectedProfile.value = list[0]
  }
}

async function loadRelease() {
  if (!selectedProfile.value) {
    releaseInfo.value = null
    return
  }
  releaseLoading.value = true
  releaseError.value = ''
  releaseInfo.value = null
  try {
    const info = await java.getRelease(
      selectedDistribution.value,
      selectedProfile.value.java_name,
    )
    releaseInfo.value = info
  } catch (e) {
    releaseError.value = e.message || String(e)
  } finally {
    releaseLoading.value = false
  }
}

async function ensureProgressListener() {
  if (!unlistenProgress) {
    unlistenProgress = await java.onProgress((payload) => {
      if (installing.value && payload) {
        const phase = payload.phase || ''
        installProgress.value = {
          received: payload.received || 0,
          total: payload.total || 0,
          phase,
        }
        if (phase === 'downloading') {
          const now = performance.now()
          speedWindow.push({ t: now, r: payload.received || 0 })
          const cutoff = now - 4000
          while (speedWindow.length > 0 && speedWindow[0].t < cutoff) {
            speedWindow.shift()
          }
        } else {
          speedWindow = []
        }
      }
    })
  }
}

async function checkAlreadyInstalled(javaName) {
  try {
    const list = await java.listInstallations()
    return Boolean(list.find((i) => i.java_name === javaName))
  } catch {
    return false
  }
}

async function installJava() {
  if (!selectedProfile.value) return

  const javaName = selectedProfile.value.java_name
  const exists = await checkAlreadyInstalled(javaName)
  if (exists) {
    const ok = confirm(
      `Java distribution "${javaName}" is already installed.\n\n` +
        `Do you want to remove the existing one and install the latest build from ${selectedDistro.value?.label || selectedDistribution.value}?`,
    )
    if (!ok) return
    try {
      await java.remove(javaName)
    } catch (e) {
      lastResult.value = { type: 'error', message: `Failed to remove existing: ${e.message || e}` }
      return
    }
  }

  installing.value = true
  installProgress.value = { received: 0, total: 0, phase: 'starting' }
  speedWindow = []
  lastResult.value = null
  error.value = ''
  await ensureProgressListener()
  try {
    const result = await java.install(selectedDistribution.value, javaName)
    lastResult.value = { type: 'success', info: result }
    await loadRelease()
  } catch (e) {
    lastResult.value = { type: 'error', message: e.message || String(e) }
  } finally {
    installing.value = false
  }
}

function pickDistribution(id) {
  if (selectedDistribution.value === id || installing.value) return
  selectedDistribution.value = id
  pickDefaultProfile()
  loadRelease()
}

function pickProfile(p) {
  if (installing.value) return
  selectedProfile.value = p
  loadRelease()
}

function resetToPicker() {
  lastResult.value = null
  installProgress.value = { received: 0, total: 0, phase: '' }
  speedWindow = []
}

onMounted(() => {
  loadAll()
})

onBeforeUnmount(() => {
  if (unlistenProgress) {
    unlistenProgress()
    unlistenProgress = null
  }
})
</script>

<template>
  <div class="view">
    <BackButton @click="$emit('back')" />
    <h2 class="section-title">{{ $t('java.title') }}</h2>

    <div v-if="state === 'loading'" class="status-block">
      <div class="spinner" />
      <p class="status-text">{{ $t('java.loading') }}</p>
    </div>

    <div v-else-if="state === 'error' && launcherProfiles.length === 0" class="status-block">
      <div class="result-icon result-icon--error">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </div>
      <p class="status-text">{{ error }}</p>
      <div class="start-button start-button--small" @click="loadAll">
        <span class="start-label">{{ $t('java.retry') }}</span>
      </div>
    </div>

    <div v-else class="select-block">
      <p class="page-description">{{ $t('java.installNewHint') }}</p>

      <div v-if="error" class="error-text">{{ error }}</div>

      <div v-if="launcherProfiles.length === 0" class="status-block">
        <p class="status-text">{{ $t('java.launcherEmpty') }}</p>
      </div>

      <template v-else-if="!installing && !lastResult">
        <p class="page-info">{{ $t('java.distribution') }}</p>
        <div class="source-toggle">
          <button
            v-for="d in distributions"
            :key="d.id"
            :class="['source-btn', { active: selectedDistribution === d.id }]"
            @click="pickDistribution(d.id)"
          >
            {{ d.label }}
          </button>
        </div>

        <p class="page-info">{{ $t('java.profile') }}</p>
        <div class="buttons-grid">
          <NavigationCard
            v-for="p in compatibleProfiles"
            :key="p.java_name"
            :label="`Java ${p.major} ${p.kind.toUpperCase()}`"
            @click="pickProfile(p)"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
          </NavigationCard>
        </div>

        <div v-if="selectedProfile" class="current-file">
          <div class="current-file-label">{{ $t('java.profile') }}</div>
          <div class="current-file-name">{{ selectedProfile.java_name }}</div>
          <div class="current-file-version">
            {{ selectedDistro?.label }} · {{ selectedProfile.version }}+{{ selectedProfile.build }} · {{ selectedProfile.usage_count }}×
          </div>
        </div>

        <div v-if="releaseLoading" class="status-text">{{ $t('java.fetchingRelease') }}</div>
        <div v-else-if="releaseError" class="error-text">{{ releaseError }}</div>

        <div
          v-else-if="releaseInfo"
          class="start-button"
          @click="installJava"
        >
          <div class="icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          </div>
          <span class="start-label">{{ $t('java.install') }}</span>
        </div>
      </template>

      <template v-else-if="installing">
        <div class="current-file">
          <div class="current-file-label">{{ $t('java.profile') }}</div>
          <div class="current-file-name">{{ selectedProfile?.java_name }}</div>
          <div class="current-file-version">{{ selectedDistro?.label }}</div>
        </div>

        <div class="progress-section">
          <div class="progress-label">
            <span>{{ $t(`java.phase.${installProgress.phase || 'starting'}`) }}</span>
            <span v-if="installProgress.total > 0">
              {{ formatBytes(installProgress.received) }} / {{ formatBytes(installProgress.total) }} · {{ progressPct }}%
            </span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: progressPct + '%' }" />
          </div>
          <div v-if="installProgress.phase === 'downloading' && (speedBps > 0 || etaSeconds > 0)" class="progress-meta">
            <span v-if="speedBps > 0">{{ formatBytes(speedBps) }}/s</span>
            <span v-if="speedBps > 0 && etaSeconds > 0">·</span>
            <span v-if="etaSeconds > 0">ETA {{ formatEta(etaSeconds) }}</span>
          </div>
        </div>
      </template>

      <template v-else-if="lastResult">
        <div class="status-block">
          <div :class="['result-icon', lastResult.type === 'success' ? 'result-icon--success' : 'result-icon--error']">
            <svg v-if="lastResult.type === 'success'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </div>
          <p v-if="lastResult.type === 'success'" class="status-text success-text">
            {{ lastResult.info.java_name }}
          </p>
          <p v-else class="status-text error-text">{{ lastResult.message }}</p>
          <div class="start-button start-button--small" @click="resetToPicker">
            <span class="start-label">{{ $t('common.back') }}</span>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.select-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  gap: 0.5rem;
}

.page-description {
  font-family: 'Inter', sans-serif;
  font-size: 1rem;
  color: #d8c8b8;
  text-align: center;
  max-width: 600px;
  margin: 0 auto 1rem auto;
  line-height: 1.5;
}

.page-info {
  margin-top: 0.8rem;
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.95rem;
  color: #d8c8b8;
  letter-spacing: 0.3px;
  opacity: 0.85;
}

.source-toggle {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  justify-content: center;
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

.source-btn:hover {
  border-color: rgba(255, 255, 255, 0.3);
}

.source-btn.active {
  border-color: #ffd700;
  background: rgba(255, 215, 0, 0.15);
  color: #ffd700;
}

.current-file {
  background-color: var(--card-bg);
  border-radius: 12px;
  padding: 1rem 1.25rem;
  border: 1px solid rgba(255, 215, 0, 0.2);
  width: 100%;
  max-width: 500px;
  margin-top: 0.8rem;
}

.current-file-label {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.9rem;
  color: #d8c8b8;
  margin-bottom: 0.3rem;
  letter-spacing: 0.5px;
}

.current-file-name {
  font-family: 'Inter', sans-serif;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--text-color);
  word-break: break-all;
}

.current-file-version {
  font-family: 'Inter', sans-serif;
  font-size: 0.85rem;
  color: #ffd700;
  margin-top: 0.3rem;
}

.progress-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  width: 100%;
  max-width: 500px;
  margin-top: 0.8rem;
}

.progress-label {
  display: flex;
  justify-content: space-between;
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.95rem;
  color: #d8c8b8;
  letter-spacing: 0.3px;
}

.progress-meta {
  display: flex;
  gap: 0.4rem;
  font-family: 'Inter', sans-serif;
  font-size: 0.8rem;
  color: #ffd700;
  opacity: 0.85;
  letter-spacing: 0.3px;
  text-align: right;
  justify-content: flex-end;
}

.progress-bar {
  width: 100%;
  height: 14px;
  background-color: rgba(255, 255, 255, 0.08);
  border-radius: 7px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #ffd700, #ffaa00);
  border-radius: 7px;
  transition: width 0.15s ease;
  box-shadow: 0 0 10px rgba(255, 215, 0, 0.4);
}

.start-button {
  background-color: var(--card-bg);
  border: 2px solid #ffd700;
  border-radius: 16px;
  padding: 1.2rem 2.5rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  margin-top: 0.8rem;
}

.start-button:hover {
  background-color: var(--card-hover);
  transform: translateY(-2px);
  box-shadow: 0 6px 12px rgba(0, 0, 0, 0.15);
}

.start-button:active {
  transform: translateY(0);
}

.start-button--small {
  margin-top: 1.5rem;
  padding: 0.8rem 2rem;
}

.start-label {
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 1.2rem;
  font-weight: 600;
  color: var(--text-color);
  letter-spacing: 0.5px;
}

.status-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  max-width: 600px;
  padding: 2rem 0;
  gap: 0.5rem;
}

.status-text {
  font-family: 'Inter', sans-serif;
  font-size: 1rem;
  color: #d8c8b8;
  margin: 1rem 0;
  text-align: center;
}

.error-text {
  font-family: 'Inter', sans-serif;
  font-size: 0.95rem;
  color: #ff6b6b;
  margin: 0.5rem 0;
  text-align: center;
  max-width: 500px;
}

.success-text {
  color: #51cf66;
}

.result-icon {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 0.5rem;
}

.result-icon svg {
  width: 36px;
  height: 36px;
}

.result-icon--success {
  background-color: rgba(81, 207, 102, 0.15);
  color: #51cf66;
}

.result-icon--error {
  background-color: rgba(255, 107, 107, 0.15);
  color: #ff6b6b;
}

.spinner {
  width: 48px;
  height: 48px;
  border: 4px solid rgba(255, 215, 0, 0.2);
  border-top-color: #ffd700;
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
