import { fetch } from '@tauri-apps/plugin-http'

const BASE_URL = 'https://api.curseforge.com/v1'

export function useCurseForgeApi() {
  const apiKey = '$2a$10$tkv4ACwWZEm.aNczS.UjqON7Lyl9O.gZZeTj.Q0dDV0Cink0U0K8C' //I don't care if its exposed or not.


  function getHeaders() {
    return {
      'x-api-key': apiKey,
      'Accept': 'application/json',
    }
  }

  function hasApiKey() {
    return apiKey.length > 0
  }

  async function searchMods(query, gameVersion, index = 0, pageSize = 16) {
    if (!hasApiKey()) {
      throw new Error('CurseForge API key not set')
    }

    const params = new URLSearchParams({
      gameId: '432',
      classId: '6',
      searchFilter: query || '',
      pageSize: String(pageSize),
      index: String(index),
      sortField: '2',
      sortOrder: 'desc',
    })
    if (gameVersion) params.set('gameVersion', gameVersion)
    if (gameVersion) params.set('modLoaderType', '4')

    const res = await fetch(`${BASE_URL}/mods/search?${params}`, { headers: getHeaders() })
    if (!res.ok) throw new Error(`CurseForge search failed: ${res.status}`)
    const json = await res.json()
    return json
  }

  async function getMod(modId) {
    if (!hasApiKey()) throw new Error('CurseForge API key not set')

    const res = await fetch(`${BASE_URL}/mods/${modId}`, { headers: getHeaders() })
    if (!res.ok) throw new Error(`CurseForge mod fetch failed: ${res.status}`)
    const json = await res.json()
    return json.data
  }

  return { searchMods, hasApiKey, getMod }
}
