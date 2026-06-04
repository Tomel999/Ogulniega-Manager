<script setup>
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";
import BackButton from "../components/BackButton.vue";
import NavigationCard from "../components/NavigationCard.vue";

defineEmits(['back']);

const REPO_OWNER = "Tomel999";
const REPO_NAME = "ogulniega-mods";
const REPO_BRANCH = "main";
const GITHUB_API = "https://api.github.com";
const USER_AGENT = "Ogulniega-Manager";

const state = ref("loading");
const availableVersions = ref([]);
const loadError = ref("");
const selectedVersion = ref("");

const VERSIONS_PER_PAGE = 12;
const versionPage = ref(1);

const currentFile = ref("");
const currentSource = ref("");
const fileReceived = ref(0);
const fileTotal = ref(0);
const overallReceived = ref(0);
const overallTotal = ref(0);
const completedFiles = ref(0);
const totalFiles = ref(0);
const failedFiles = ref([]);
const doneMessage = ref("");

const filePercent = computed(() => {
  if (fileTotal.value <= 0) return 0;
  return Math.min(100, Math.round((fileReceived.value / fileTotal.value) * 100));
});

const overallPercent = computed(() => {
  if (overallTotal.value <= 0) return 0;
  return Math.min(100, Math.round((overallReceived.value / overallTotal.value) * 100));
});

const totalVersionPages = computed(() =>
  Math.max(1, Math.ceil(availableVersions.value.length / VERSIONS_PER_PAGE))
);

const visibleVersionPages = computed(() => {
  const pages = [];
  const last = totalVersionPages.value;
  const current = versionPage.value;
  if (last <= 7) {
    for (let i = 1; i <= last; i += 1) pages.push(i);
    return pages;
  }
  pages.push(1);
  const start = Math.max(2, current - 1);
  const end = Math.min(last - 1, current + 1);
  if (start > 2) pages.push("...");
  for (let i = start; i <= end; i += 1) pages.push(i);
  if (end < last - 1) pages.push("...");
  pages.push(last);
  return pages;
});

const pagedVersions = computed(() => {
  const start = (versionPage.value - 1) * VERSIONS_PER_PAGE;
  return availableVersions.value.slice(start, start + VERSIONS_PER_PAGE);
});

function goToVersionPage(n) {
  if (n >= 1 && n <= totalVersionPages.value) versionPage.value = n;
}

function formatBytes(n) {
  if (!n || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
}

async function githubFetch(url) {
  return await fetch(url, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": USER_AGENT,
    },
  });
}

async function listRepoVersions() {
  const url = `${GITHUB_API}/repos/${REPO_OWNER}/${REPO_NAME}/contents/versions?ref=${REPO_BRANCH}`;
  const res = await githubFetch(url);
  if (res.status === 404) return [];
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`GitHub API ${res.status}: ${text || res.statusText}`);
  }
  const data = await res.json();
  if (!Array.isArray(data)) return [];
  return data
    .filter((item) => item.type === "dir")
    .map((item) => item.name)
    .sort();
}

async function listVersionFolderFiles(version) {
  const url = `${GITHUB_API}/repos/${REPO_OWNER}/${REPO_NAME}/contents/versions/${encodeURIComponent(version)}?ref=${REPO_BRANCH}`;
  const res = await githubFetch(url);
  if (res.status === 404) return [];
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`GitHub API ${res.status}: ${text || res.statusText}`);
  }
  const data = await res.json();
  return Array.isArray(data) ? data : [];
}

async function listReleases() {
  const all = [];
  let page = 1;
  while (page <= 5) {
    const url = `${GITHUB_API}/repos/${REPO_OWNER}/${REPO_NAME}/releases?per_page=100&page=${page}`;
    const res = await githubFetch(url);
    if (res.status === 404) break;
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      throw new Error(`GitHub API ${res.status}: ${text || res.statusText}`);
    }
    const data = await res.json();
    if (!Array.isArray(data) || data.length === 0) break;
    all.push(...data);
    if (data.length < 100) break;
    page += 1;
  }
  return all;
}

function releaseMatchesVersion(release, version) {
  if (!release || !version) return false;
  const v = version;
  if (release.name && release.name === v) return true;
  const tag = (release.tag_name || "").replace(/^mods-/, "");
  if (tag && tag === v) return true;
  if ((release.tag_name || "").endsWith("-" + v)) return true;
  return false;
}

async function fetchVersionFolderMods(version) {
  const items = await listVersionFolderFiles(version);
  const result = [];
  for (const item of items) {
    if (item.type !== "file") continue;
    if (!item.name.toLowerCase().endsWith(".jar")) continue;
    if (!item.download_url) continue;
    result.push({
      version,
      filename: item.name,
      url: item.download_url,
      size: item.size || 0,
      source: "repo",
    });
  }
  return result;
}

async function downloadFileWithProgress(url, onChunk) {
  const res = await fetch(url, {
    headers: { "User-Agent": USER_AGENT },
  });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }
  const declared = parseInt(res.headers.get("content-length") || "0", 10);
  const reader = res.body && res.body.getReader ? res.body.getReader() : null;
  if (!reader) {
    const buf = new Uint8Array(await res.arrayBuffer());
    onChunk(buf.length, buf.length);
    return buf;
  }
  const chunks = [];
  let received = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    received += value.length;
    onChunk(received, declared);
  }
  const totalLen = chunks.reduce((s, c) => s + c.length, 0);
  const out = new Uint8Array(totalLen);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  return out;
}

function resetProgress() {
  currentFile.value = "";
  currentSource.value = "";
  fileReceived.value = 0;
  fileTotal.value = 0;
  overallReceived.value = 0;
  overallTotal.value = 0;
  completedFiles.value = 0;
  totalFiles.value = 0;
  failedFiles.value = [];
  doneMessage.value = "";
}

async function loadVersions() {
  state.value = "loading";
  loadError.value = "";
  versionPage.value = 1;
  try {
    availableVersions.value = await listRepoVersions();
    if (availableVersions.value.length === 0) {
      state.value = "empty";
    } else {
      state.value = "selecting";
    }
  } catch (e) {
    loadError.value = String(e.message || e);
    state.value = "loadError";
  }
}

async function pickVersion(version) {
  selectedVersion.value = version;
  await startDownload();
}

async function startDownload() {
  resetProgress();
  state.value = "downloading";

  const version = selectedVersion.value;
  const downloadList = [];

  try {
    const mods = await fetchVersionFolderMods(version);
    for (const m of mods) downloadList.push(m);
  } catch (e) {
    failedFiles.value.push(`versions/${version}: ${e.message || e}`);
  }

  try {
    const releases = await listReleases();
    for (const release of releases) {
      if (!releaseMatchesVersion(release, version)) continue;
      for (const asset of release.assets || []) {
        if (!asset || !asset.name || !asset.browser_download_url) continue;
        if (!asset.name.toLowerCase().endsWith(".jar")) continue;
        downloadList.push({
          version,
          filename: asset.name,
          url: asset.browser_download_url,
          size: asset.size || 0,
          source: "release",
        });
      }
    }
  } catch (e) {
    console.warn("Releases fetch failed:", e);
  }

  totalFiles.value = downloadList.length;
  overallTotal.value = downloadList.reduce((s, i) => s + (i.size || 0), 0);

  if (downloadList.length === 0) {
    state.value = "done";
    doneMessage.value = "noModsFound";
    return;
  }

  for (const item of downloadList) {
    currentFile.value = item.filename;
    currentSource.value = item.source;
    fileReceived.value = 0;
    fileTotal.value = item.size || 0;

    let prevReceived = 0;
    try {
      const data = await downloadFileWithProgress(item.url, (received, total) => {
        fileReceived.value = received;
        if (total > 0) fileTotal.value = total;
        const delta = received - prevReceived;
        if (delta > 0) {
          overallReceived.value += delta;
          prevReceived = received;
        }
      });
      await invoke("write_preinstalled_mod_file", {
        gameVersion: item.version,
        filename: item.filename,
        data: Array.from(data),
      });
      completedFiles.value += 1;
    } catch (e) {
      failedFiles.value.push(`${item.version}/${item.filename}: ${e.message || e}`);
    }
  }

  state.value = "done";
  doneMessage.value =
    failedFiles.value.length === 0
      ? "allSuccess"
      : failedFiles.value.length === totalFiles.value
        ? "allFailed"
        : "partial";
}

function backToSelector() {
  resetProgress();
  selectedVersion.value = "";
  state.value = "selecting";
}

onMounted(() => {
  loadVersions();
});
</script>

<template>
  <div class="view view--full">
    <BackButton @click="$emit('back')" />
    <h2 class="section-title">{{ $t('alternativeDownload.title') }}</h2>

    <div v-if="state === 'loading'" class="status-block">
      <div class="spinner" />
      <p class="status-text">{{ $t('alternativeDownload.loadingVersions') }}</p>
    </div>

    <div v-else-if="state === 'loadError'" class="status-block">
      <div class="result-icon result-icon--error">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </div>
      <p class="status-text">{{ $t('alternativeDownload.loadError') }}</p>
      <p v-if="loadError" class="status-text status-text--small">{{ loadError }}</p>
      <div class="start-button start-button--small" @click="loadVersions">
        <span class="start-label">{{ $t('alternativeDownload.retry') }}</span>
      </div>
    </div>

    <div v-else-if="state === 'empty'" class="status-block">
      <div class="result-icon result-icon--warn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      </div>
      <p class="status-text">{{ $t('alternativeDownload.noVersionsInRepo') }}</p>
    </div>

    <div v-else-if="state === 'selecting'" class="select-block">
      <p class="page-description">{{ $t('alternativeDownload.selectVersion') }}</p>
      <div class="buttons-grid">
        <NavigationCard
          v-for="version in pagedVersions"
          :key="version"
          :label="version"
          @click="pickVersion(version)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>
        </NavigationCard>
      </div>
      <div v-if="totalVersionPages > 1" class="pagination">
        <button class="page-btn" :disabled="versionPage <= 1" @click="goToVersionPage(versionPage - 1)">‹</button>
        <button
          v-for="page in visibleVersionPages"
          :key="`vp-${page}`"
          :class="['page-btn', { active: page === versionPage, ellipsis: page === '...' }]"
          :disabled="page === '...'"
          @click="page !== '...' && goToVersionPage(page)"
        >{{ page }}</button>
        <button class="page-btn" :disabled="versionPage >= totalVersionPages" @click="goToVersionPage(versionPage + 1)">›</button>
      </div>
      <div class="page-info">{{ versionPage }} / {{ totalVersionPages }} · {{ availableVersions.length }} {{ $t('alternativeDownload.versionsCount') }}</div>
    </div>

    <div v-else-if="state === 'downloading'" class="progress-block">
      <div class="current-file">
        <div class="current-file-label">{{ $t('alternativeDownload.currentFile') }} · v{{ selectedVersion }}</div>
        <div class="current-file-name">{{ currentFile }}</div>
        <div class="current-file-version">{{ currentSource === 'release' ? $t('alternativeDownload.fromRelease') : $t('alternativeDownload.fromRepo') }}</div>
      </div>

      <div class="progress-section">
        <div class="progress-label">
          <span>{{ $t('alternativeDownload.fileProgress') }}</span>
          <span>{{ formatBytes(fileReceived) }} / {{ formatBytes(fileTotal) }} · {{ filePercent }}%</span>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: filePercent + '%' }" />
        </div>
      </div>

      <div class="progress-section">
        <div class="progress-label">
          <span>{{ $t('alternativeDownload.overallProgress') }}</span>
          <span>{{ completedFiles }} / {{ totalFiles }} · {{ overallPercent }}%</span>
        </div>
        <div class="progress-bar progress-bar--overall">
          <div class="progress-fill" :style="{ width: overallPercent + '%' }" />
        </div>
      </div>
    </div>

    <div v-else-if="state === 'done'" class="status-block">
      <div :class="['result-icon', { 'result-icon--success': failedFiles.length === 0, 'result-icon--warn': failedFiles.length > 0 && completedFiles > 0, 'result-icon--error': doneMessage === 'allFailed' || doneMessage === 'noModsFound' }]">
        <svg v-if="failedFiles.length === 0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      </div>
      <p v-if="doneMessage === 'noModsFound'" class="status-text">{{ $t('alternativeDownload.noModsFound') }}</p>
      <p v-else-if="doneMessage === 'allSuccess'" class="status-text">{{ $t('alternativeDownload.allSuccess', { count: completedFiles }) }}</p>
      <p v-else-if="doneMessage === 'allFailed'" class="status-text">{{ $t('alternativeDownload.allFailed') }}</p>
      <p v-else class="status-text">{{ $t('alternativeDownload.partialSuccess', { ok: completedFiles, fail: failedFiles.length }) }}</p>
      <ul v-if="failedFiles.length > 0" class="error-list">
        <li v-for="(msg, idx) in failedFiles" :key="idx">{{ msg }}</li>
      </ul>
      <div class="done-actions">
        <div class="start-button start-button--small" @click="backToSelector">
          <span class="start-label">{{ $t('alternativeDownload.pickAnother') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page-description {
  font-family: 'Inter', sans-serif;
  font-size: 1rem;
  color: #d8c8b8;
  text-align: center;
  max-width: 600px;
  margin: 0 auto 1.5rem auto;
  line-height: 1.5;
}

.select-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.start-button {
  background-color: var(--card-bg);
  border: 2px solid #ffd700;
  border-radius: 16px;
  padding: 1.5rem 2.5rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
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

.status-text {
  font-family: 'Inter', sans-serif;
  font-size: 1rem;
  color: #d8c8b8;
  text-align: center;
  margin: 0.5rem 0;
}

.status-text--small {
  font-size: 0.85rem;
  opacity: 0.7;
  word-break: break-all;
  max-width: 500px;
}

.result-icon {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
}

.result-icon svg {
  width: 36px;
  height: 36px;
}

.result-icon--success {
  background-color: rgba(81, 207, 102, 0.15);
  color: #51cf66;
}

.result-icon--warn {
  background-color: rgba(255, 215, 0, 0.15);
  color: #ffd700;
}

.result-icon--error {
  background-color: rgba(255, 107, 107, 0.15);
  color: #ff6b6b;
}

.error-list {
  list-style: none;
  padding: 0;
  margin: 1rem 0 0 0;
  width: 100%;
  max-width: 500px;
  max-height: 160px;
  overflow-y: auto;
  font-family: 'Inter', sans-serif;
  font-size: 0.85rem;
  color: #ff6b6b;
}

.error-list li {
  background: rgba(255, 107, 107, 0.08);
  border-left: 3px solid #ff6b6b;
  padding: 0.4rem 0.8rem;
  margin-bottom: 0.3rem;
  border-radius: 4px;
  word-break: break-all;
}

.progress-block {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  width: 100%;
  max-width: 600px;
  gap: 1.5rem;
}

.current-file {
  background-color: var(--card-bg);
  border-radius: 12px;
  padding: 1rem 1.25rem;
  border: 1px solid rgba(255, 215, 0, 0.2);
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
}

.progress-label {
  display: flex;
  justify-content: space-between;
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.95rem;
  color: #d8c8b8;
  letter-spacing: 0.3px;
}

.progress-bar {
  width: 100%;
  height: 14px;
  background-color: rgba(255, 255, 255, 0.08);
  border-radius: 7px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.progress-bar--overall {
  height: 18px;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #ffd700, #ffaa00);
  border-radius: 7px;
  transition: width 0.15s ease;
  box-shadow: 0 0 10px rgba(255, 215, 0, 0.4);
}

.done-actions {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
  justify-content: center;
}

.pagination {
  display: flex;
  gap: 0.4rem;
  margin-top: 1.5rem;
  flex-wrap: wrap;
  justify-content: center;
}

.page-btn {
  min-width: 36px;
  height: 36px;
  padding: 0 0.5rem;
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

.page-info {
  margin-top: 0.6rem;
  font-family: 'Pixelify Sans', sans-serif;
  font-size: 0.9rem;
  color: #d8c8b8;
  letter-spacing: 0.3px;
  opacity: 0.85;
}
</style>
