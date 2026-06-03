import { fetch } from '@tauri-apps/plugin-http'

const BASE_URL = 'https://api.modrinth.com/v2'
const USER_AGENT = 'ogulniega-manager/0.1.0'

function buildFacets(gameVersion) {
  const facets = []
  if (gameVersion) facets.push([`versions:${gameVersion}`])
  facets.push(['categories:fabric'])
  facets.push(['project_type:mod'])
  return facets
}

export function useModrinthApi() {
  function getHeaders() {
    return {
      'User-Agent': USER_AGENT,
      'Content-Type': 'application/json',
    }
  }

  async function searchMods(query, gameVersion, offset = 0, limit = 16) {
    const params = new URLSearchParams()
    if (query) params.set('query', query)
    const facets = buildFacets(gameVersion)
    if (facets.length) params.set('facets', JSON.stringify(facets))
    params.set('limit', String(limit))
    if (offset > 0) params.set('offset', String(offset))

    const res = await fetch(`${BASE_URL}/search?${params}`, { headers: getHeaders() })
    if (!res.ok) throw new Error(`Modrinth search failed: ${res.status}`)
    return res.json()
  }

  async function getProjectVersions(slug, gameVersion) {
    const params = new URLSearchParams()
    params.set('loaders', JSON.stringify(['fabric']))
    if (gameVersion) params.set('game_versions', JSON.stringify([gameVersion]))
    params.set('include_changelog', 'false')

    const res = await fetch(`${BASE_URL}/project/${slug}/version?${params}`, { headers: getHeaders() })
    if (!res.ok) throw new Error(`Modrinth versions fetch failed: ${res.status}`)
    return res.json()
  }

  async function getProject(projectId) {
    const res = await fetch(`${BASE_URL}/project/${projectId}`, { headers: getHeaders() })
    if (!res.ok) throw new Error(`Modrinth project fetch failed: ${res.status}`)
    return res.json()
  }

  return { searchMods, getProjectVersions, getProject }
}
