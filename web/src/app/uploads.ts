import { computed, markRaw, ref } from 'vue'
import { defineStore } from 'pinia'
import { observeEvents } from './events'
import { aggregate } from './aggregate'
export interface Transfer { id: string; path: string; total_bytes: number | null; transferred_bytes: number; state: 'queued' | 'uploading' | 'publishing' | 'completed' | 'failed' | 'cancelled'; bytes_per_second: number | null; eta_seconds: number | null; failure: string | null }
export interface QueueItem { id: string; key: string; file: File; path: string; state: Transfer['state'] | 'unconfirmed'; transferred: number; total: number | null; speed: number | null; failure: string | null; policy: 'ask' | 'replace' | 'auto_rename' | 'reject'; controller?: AbortController; sent: boolean }
function token() { return [...crypto.getRandomValues(new Uint8Array(16))].map(value => value.toString(16).padStart(2,'0')).join('') }
export const useUploads = defineStore('uploads', () => {
  const items = ref<QueueItem[]>([])
  const limit = ref(1)
  const enabled = ref(false)
  const problem = ref('')
  const progress = computed(() => aggregate(items.value))
  const speed = computed(() => items.value.filter(item => item.state === 'uploading').reduce((sum,item) => sum + (item.speed ?? 0),0))
  const eta = computed(() => progress.value.total !== null && speed.value > 0 ? Math.ceil((progress.value.total - progress.value.transferred)/speed.value) : null)
  let inflight = 0
  let stopEvents: (() => void) | undefined
  function apply(transfer: Transfer) {
    const item = items.value.find(item => item.id === transfer.id)
    if (!item) return
    item.state = transfer.state; item.transferred = transfer.transferred_bytes; item.total = transfer.total_bytes ?? item.total; item.speed = transfer.bytes_per_second; item.failure = transfer.failure
    item.path = transfer.path
  }
  async function refresh() {
    try {
      const response = await fetch('/api/v1/transfers')
      if (!response.ok) { problem.value = 'Transfer status is unavailable. Reconnect or check the destination before retrying.'; return }
      const transfers = await response.json() as Transfer[]
      problem.value = ''
      const known = new Set(transfers.map(transfer => transfer.id))
      for (const transfer of transfers) apply(transfer)
      for (const item of items.value) {
        if (item.sent && !item.controller && !known.has(item.id) && ['queued','uploading','publishing'].includes(item.state)) {
          item.state = 'unconfirmed'; item.failure = 'STATUS_UNKNOWN'; item.speed = null
        }
      }
      return known
    } catch { problem.value = 'Could not refresh transfer status. Reconnecting…' }
  }
  function start(concurrency: number) {
    limit.value = Math.max(1,concurrency)
    if (stopEvents) return
    stopEvents = observeEvents(event => {
      if (event.type === 'transfer.resync') void refresh()
      if (event.type === 'transfer.created' || event.type === 'transfer.updated') apply(JSON.parse(event.data) as Transfer)
    })
  }
  function stop() { stopEvents?.(); stopEvents = undefined; for (const item of items.value) item.controller?.abort() }
  function enqueue(files: File[], parent: string) {
    if (!enabled.value) return
    if (items.value.length + files.length > 128) { problem.value = 'The queue holds up to 128 files. Clear finished items first.'; return }
    for (const file of files) items.value.push({ id: token(), key: token(), file: markRaw(file), path: `${parent === '/' ? '' : parent}/${file.name}`, state: 'queued', transferred: 0, total: file.size, speed: null, failure: null, policy: 'ask', sent: false })
    pump()
  }
  function pump() {
    if (!enabled.value) return
    while (inflight < Math.min(limit.value, 6)) {
      const item = items.value.find(item => item.state === 'queued' && !item.sent)
      if (!item) break
      item.sent = true; inflight++; void send(item)
    }
  }
  async function send(item: QueueItem) {
    item.controller = markRaw(new AbortController())
    try {
      const response = await fetch(`/api/v1/uploads?${new URLSearchParams({ path: item.path, policy: item.policy })}`, { method: 'POST', body: item.file, signal: item.controller.signal, headers: { 'Content-Type': 'application/octet-stream', 'X-SplitShare-Request': '1', 'X-Transfer-ID': item.id, 'X-Transfer-Key': item.key } })
      if (response.ok) apply(await response.json() as Transfer)
      else {
        const body = await response.json()
        if (item.state !== 'completed') { item.state = body.error.code === 'TRANSFER_CANCELLED' ? 'cancelled' : 'failed'; item.failure = body.error.code }
      }
    } catch {
      // A lost response is not proof of failure: reconcile server state first.
      const known = await refresh()
      if (!known?.has(item.id) && !['completed','cancelled'].includes(item.state)) {
        item.state = 'unconfirmed'; item.failure = 'STATUS_UNKNOWN'; item.speed = null
      }
    } finally { item.controller = undefined; inflight--; pump() }
  }
  async function cancel(item: QueueItem) {
    if (!item.sent) { item.state = 'cancelled'; pump(); return }
    try {
      const response = await fetch(`/api/v1/transfers/${item.id}`, { method: 'DELETE', headers: { 'X-SplitShare-Request': '1', 'X-Transfer-Key': item.key } })
      if (response.status === 404) { item.controller?.abort(); await refresh() }
      else if (!response.ok) { problem.value = (await response.json()).error.message }
      else { await refresh() }
    } catch { problem.value = 'Cancellation could not be confirmed. Check the connection and retry.' }
  }
  function retry(item: QueueItem, policy: QueueItem['policy'] = item.policy) {
    if (!enabled.value || item.controller || !['failed','cancelled','unconfirmed'].includes(item.state)) return
    if (item.state === 'unconfirmed') policy = 'ask' // Never overwrite after an uncertain response.
    item.id = token(); item.key = token(); item.transferred = 0; item.failure = null; item.speed = null; item.state = 'queued'; item.policy = policy; item.sent = false
    pump()
  }
  function setEnabled(value: boolean) {
    enabled.value = value
    if (!value) { for (const item of items.value) { if (!item.sent && item.state === 'queued') item.state = 'cancelled' } }
    else pump()
  }
  function keepExisting(item: QueueItem) { if (item.state === 'failed' && item.failure === 'CONFLICT') { item.state = 'cancelled'; item.failure = null } }
  function clear() { items.value = items.value.filter(item => !['completed','failed','cancelled'].includes(item.state) || item.controller) }
  return { items, progress, speed, eta, limit, problem, start, stop, setEnabled, enqueue, cancel, retry, keepExisting, clear, refresh }
})
