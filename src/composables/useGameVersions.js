import { invoke } from '@tauri-apps/api/core'

export function useGameVersions() {
  async function getVersions() {
    const versions = await invoke('list_game_versions')
    return versions
  }

  return { getVersions }
}
