<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import { request } from '../app/api'
import { showDialog, closeDialog } from '../app/dialogs'
interface Entry { id:number; timestamp_ms:number; level:string; target:string; message:string; fields:Record<string,string> }
const dialog = ref<HTMLDialogElement>()
const entries = ref<Entry[]>([])
const search = ref('')
const level = ref('all')
const error = ref('')
const connected = ref(false)
const loading = ref(false)
const clearing = ref(false)
const visible = computed(() => entries.value.filter(entry => (level.value === 'all' || entry.level === level.value) && `${entry.message} ${entry.target} ${Object.values(entry.fields).join(' ')}`.toLocaleLowerCase().includes(search.value.toLocaleLowerCase())).slice().reverse())
let source: EventSource | undefined
let controller: AbortController | undefined
let scheduled: ReturnType<typeof setTimeout> | undefined
let generation = 0
let fetching = false, dirty = false
function schedule() {
  dirty = true
  if (!scheduled && !fetching && dialog.value?.open) scheduled = setTimeout(() => { scheduled = undefined; void refresh() },250)
}
async function refresh() {
  if (!dialog.value?.open || fetching) return
  const current = generation
  fetching = true; dirty = false; controller = new AbortController()
  try {
    const result = await request<Entry[]>('/host/logs','GET',undefined,controller.signal)
    if (current === generation) { entries.value = result; error.value = '' }
  } catch (cause) { if (current === generation && (cause as Error).name !== 'AbortError') error.value = (cause as Error).message }
  finally { if (current === generation) { fetching = false; loading.value = false; if (dirty) schedule() } }
}
function close() {
  generation++; source?.close(); source = undefined; controller?.abort(); controller = undefined
  clearTimeout(scheduled); scheduled = undefined; fetching = false; dirty = false; connected.value = false; loading.value = false
  closeDialog(dialog.value)
}
function open() {
  if (dialog.value?.open) return
  entries.value = []; search.value = ''; level.value = 'all'; error.value = ''; loading.value = true
  showDialog(dialog.value)
  source = new EventSource('/api/v1/host/logs/events')
  source.onopen = () => { connected.value = true; schedule() }
  source.onerror = () => { connected.value = false }
  source.addEventListener('logs.changed',schedule)
  void refresh()
}
async function clear() {
  const current = generation
  clearing.value = true
  try { await request('/host/logs','DELETE'); if (current === generation) schedule() }
  catch (cause) { if (current === generation) error.value = (cause as Error).message }
  finally { clearing.value = false }
}
onUnmounted(close)
defineExpose({open})
</script>
<template>
  <dialog ref="dialog" class="logs-dialog" aria-label="Host logs" @cancel.prevent="close">
    <h2>Host logs</h2>
    <p>Latest 500 entries from this run. Newest first. Cleared when SplitShare exits.</p>
    <p role="status">{{ loading ? 'Loading logs…' : connected ? 'Live updates connected' : 'Live updates disconnected. Reconnecting…' }}</p>
    <div class="log-filters">
      <label>Search logs<input v-model="search" type="search" placeholder="Message or transfer ID" /></label>
      <label>Log level filter<select v-model="level"><option value="all">All levels</option><option>ERROR</option><option>WARN</option><option>INFO</option><option>DEBUG</option><option>TRACE</option></select></label>
    </div>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <p v-if="!loading && !visible.length">{{ entries.length ? 'No matching logs.' : 'No logs recorded yet at the current logging level.' }}</p>
    <ol class="log-list" aria-label="Log entries" tabindex="0">
      <li v-for="entry in visible" :key="entry.id" class="log-entry">
        <div><time :datetime="new Date(entry.timestamp_ms).toISOString()">{{ new Date(entry.timestamp_ms).toLocaleString() }}</time> <strong :class="{ 'danger-text': ['ERROR','WARN'].includes(entry.level) }">{{ entry.level }}</strong></div>
        <p>{{ entry.message }}</p><small>{{ entry.target }}</small>
        <dl v-if="Object.keys(entry.fields).length"><template v-for="(value,key) in entry.fields" :key="key"><dt>{{ key }}</dt><dd>{{ value }}</dd></template></dl>
      </li>
    </ol>
    <div class="dialog-actions"><button type="button" class="button" :disabled="loading" @click="schedule">Refresh</button><button type="button" class="button" :disabled="clearing || !entries.length" @click="clear">Clear logs</button><button type="button" class="button primary" @click="close">Close logs</button></div>
  </dialog>
</template>
