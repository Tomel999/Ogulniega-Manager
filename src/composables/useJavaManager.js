import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export function useJavaManager() {
  async function getPlatform() {
    return await invoke('get_platform_info')
  }

  async function getJavaPath() {
    return await invoke('get_java_path')
  }

  async function listInstallations() {
    return await invoke('list_java_installations')
  }

  async function listDistributions() {
    return await invoke('list_distributions')
  }

  async function getLauncherProfiles() {
    return await invoke('get_launcher_profiles')
  }

  async function getRelease(distribution, javaName) {
    return await invoke('get_java_release', {
      distribution,
      javaName,
    })
  }

  async function install(distribution, javaName) {
    return await invoke('install_java', {
      distribution,
      javaName,
    })
  }

  async function remove(javaName) {
    return await invoke('delete_java', { javaName })
  }

  function onProgress(callback) {
    return listen('java-install-progress', (event) => {
      callback(event.payload)
    })
  }

  return {
    getPlatform,
    getJavaPath,
    listInstallations,
    listDistributions,
    getLauncherProfiles,
    getRelease,
    install,
    remove,
    onProgress,
  }
}
