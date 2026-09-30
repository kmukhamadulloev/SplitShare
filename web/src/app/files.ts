import { defineStore } from 'pinia'
import { ref } from 'vue'
import { observeEvents } from './events'
import { ApiError, request, type FileEntry, type Status } from './api'

export const useFiles = defineStore('files', () => {
  const status = ref<Status | null>(null)
  const path = ref('/')
  const entries = ref<FileEntry[]>([])
  const error = ref('')
  const loading = ref(false)
  const refreshing = ref(false)
  const connected = ref(false)
  const initialized = ref(false)
  const sessionRequired = ref(false)
  let revision = 0, statusRevision = 0
  let listing: AbortController | undefined, statusRequest: AbortController | undefined
  let stopEvents: (() => void) | undefined
  function clearEntries() { revision++; listing?.abort(); entries.value = []; loading.value = false }
  function failed(cause: unknown) {
    error.value = cause instanceof ApiError ? cause.message : 'Cannot reach the host. Check your connection and try again.'
    if (cause instanceof ApiError && cause.status === 401) { sessionRequired.value = true; status.value = null; clearEntries() }
  }
  async function load(destination = path.value) {
    if (!status.value?.permissions.browse) { clearEntries(); return }
    const current = ++revision
    if (path.value !== destination) { entries.value = []; path.value = destination }
    listing?.abort(); listing = new AbortController()
    loading.value = true; error.value = ''
    try {
      const result = await request<{ path: string; entries: FileEntry[] }>(`/files?${new URLSearchParams({ path: destination })}`,'GET',undefined,listing.signal)
      if (current !== revision) return
      entries.value = result.entries; path.value = result.path
    } catch (cause) { if (current === revision && (cause as Error).name !== 'AbortError') { entries.value = []; failed(cause) } }
    finally { if (current === revision) loading.value = false }
  }
  async function refreshStatus() {
    const current = ++statusRevision
    statusRequest?.abort(); statusRequest = new AbortController()
    try {
      const result = await request<Status>('/status','GET',undefined,statusRequest.signal)
      if (current !== statusRevision) return false
      status.value = result; sessionRequired.value = false; error.value = ''
      if (!result.permissions.browse) clearEntries()
      return true
    } catch (cause) {
      if (current === statusRevision && (cause as Error).name !== 'AbortError') { status.value = null; clearEntries(); failed(cause) }
      return false
    } finally { if (current === statusRevision) initialized.value = true }
  }
  let refreshRevision = 0
  async function refresh() {
    const current = ++refreshRevision
    refreshing.value = true
    try { if (await refreshStatus()) await load() }
    finally { if (current === refreshRevision) refreshing.value = false }
  }
  async function start() {
    stop()
    stopEvents = observeEvents(event => {
      if (event.type === 'session.permissions_changed' || event.type === 'filesystem.resync') void refresh()
      if (event.type === 'filesystem.changed') {
        const changed = JSON.parse(event.data) as { paths: string[] }
        if (changed.paths.includes(path.value)) void load()
      }
    }, value => { connected.value = value })
    await refresh()
  }
  function stop() { stopEvents?.(); stopEvents = undefined; connected.value = false; revision++; statusRevision++; listing?.abort(); statusRequest?.abort() }
  async function create(name: string) { await request('/directories','POST',{ parent:path.value, name }); await load() }
  async function rename(entry: FileEntry, name: string) { const parent = entry.path.slice(0,entry.path.lastIndexOf('/')); await request('/files/rename','POST',{from:entry.path,to:`${parent}/${name}`}); await load() }
  async function remove(entry: FileEntry) { await request('/files','DELETE',{path:entry.path}); await load() }
  return { status, path, entries, error, loading, refreshing, connected, initialized, sessionRequired, start, stop, refresh, refreshStatus, load, create, rename, remove }
})
