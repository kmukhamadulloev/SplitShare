import { defineStore } from 'pinia'
import { ref } from 'vue'
import { observeEvents } from './events'
import { request, type FileEntry, type Status } from './api'

export const useFiles = defineStore('files', () => {
  const status = ref<Status | null>(null)
  const path = ref('/')
  const entries = ref<FileEntry[]>([])
  const error = ref('')
  const loading = ref(false)
  const connected = ref(false)
  let revision = 0
  let stopEvents: (() => void) | undefined
  async function load(destination = path.value) {
    if (!status.value?.permissions.browse) { entries.value = []; return }
    const current = ++revision
    loading.value = true
    try {
      const result = await request<{ path: string; entries: FileEntry[] }>(`/files?${new URLSearchParams({ path: destination })}`)
      if (current !== revision) return
      entries.value = result.entries
      path.value = result.path
      error.value = ''
    } catch (cause) { if (current === revision) error.value = String((cause as Error).message) }
    finally { if (current === revision) loading.value = false }
  }
  async function refreshStatus() {
    try {
      status.value = await request<Status>('/status')
      if (!status.value.permissions.browse) { revision++; entries.value = [] }
    } catch (cause) { revision++; status.value = null; entries.value = []; error.value = (cause as Error).message }
  }
  async function start() {
    stop()

    try {
      await refreshStatus()
      if (!status.value?.sharing) return
      await load()
      stopEvents = observeEvents(event => {
        if (event.type === 'session.permissions_changed') { void refreshStatus().then(() => load()); return }
        if (event.type === 'filesystem.resync') void refreshStatus().then(() => load())
        if (event.type === 'filesystem.changed') {
          const changed = JSON.parse(event.data) as { paths: string[] }
          if (changed.paths.includes(path.value)) void load()
        }
      }, value => { connected.value = value; if (!value) void refreshStatus() })
    } catch (cause) { error.value = (cause as Error).message }
  }
  function stop() { stopEvents?.(); stopEvents = undefined; connected.value = false; revision++ }
  async function create(name: string) { await request('/directories', 'POST', { parent: path.value, name }); await load() }
  async function rename(entry: FileEntry, name: string) {
    const parent = entry.path.slice(0, entry.path.lastIndexOf('/'))
    await request('/files/rename', 'POST', { from: entry.path, to: `${parent}/${name}` }); await load()
  }
  async function remove(entry: FileEntry) { await request('/files', 'DELETE', { path: entry.path }); await load() }
  return { status, path, entries, error, loading, connected, start, stop, refreshStatus, load, create, rename, remove }
})
