<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Download, LayoutGrid, List, FolderPlus, RefreshCw, ChevronRight, ArrowUp, Search, MoreHorizontal, Trash2, X, FolderOpen, WifiOff, ShieldAlert, LoaderCircle } from '@lucide/vue'
import Transfers from './components/Transfers.vue'
import Sharing from './components/Sharing.vue'
import FileIcon from './components/FileIcon.vue'
import FileActions from './components/FileActions.vue'
import { useFiles } from './app/files'
import { downloadUrl, request, type FileEntry } from './app/api'
import { fileCategory, typeLabels, formatBytes } from './app/file-types'
import { showDialog, closeDialog } from './app/dialogs'
const files = useFiles()
const transfers = ref<InstanceType<typeof Transfers>>()
const actions = ref<InstanceType<typeof FileActions>>()
const grid = ref(false)
const search = ref('')
const selected = ref<string[]>([])
const visible = computed(() => files.entries.filter(entry => entry.name.toLocaleLowerCase().includes(search.value.toLocaleLowerCase())))
const page = ref(1)
const fileArea = ref<HTMLElement>()
const pageSize = 100
const pageCount = computed(() => Math.max(1, Math.ceil(visible.value.length / pageSize)))
const pageEntries = computed(() => visible.value.slice((page.value - 1) * pageSize, page.value * pageSize))
const selection = computed(() => files.entries.filter(entry => selected.value.includes(entry.path)))
const allSelected = computed(() => pageEntries.value.length > 0 && pageEntries.value.every(entry => selected.value.includes(entry.path)))
const crumbs = computed(() => [{name:'Shared folder',path:'/'}, ...files.path.split('/').filter(Boolean).map((name,index,parts) => ({name,path:'/'+parts.slice(0,index+1).join('/')}))])
const dialog = ref<HTMLDialogElement>()
const downloads = ref<HTMLDialogElement>()
const input = ref<HTMLInputElement>()
const action = ref<'create' | 'rename' | 'delete' | 'delete-selected'>('create')
const targets = ref<FileEntry[]>([])
const name = ref('')
const modalError = ref('')
const busy = ref(false)
const dragging = ref(false)
let dragDepth = 0
const title = computed(() => ({create:'Create folder',rename:'Rename item',delete:'Delete item','delete-selected':'Delete selected items'})[action.value])
const deleteAction = computed(() => action.value.startsWith('delete'))
const confirmation = computed(() => action.value === 'delete-selected' ? 'DELETE' : targets.value[0]?.name ?? '')
const allPermissions = computed(() => files.status && Object.values(files.status.permissions).every(Boolean))
function context(event: MouseEvent | KeyboardEvent, entry: FileEntry) { void actions.value?.open(entry,event) }
function toggleAll() {
  selected.value = allSelected.value ? selected.value.filter(path => !pageEntries.value.some(entry => entry.path === path)) : [...new Set([...selected.value,...pageEntries.value.map(entry => entry.path)])]
}
async function open(kind: typeof action.value, entry?: FileEntry) {
  action.value = kind; targets.value = kind === 'delete-selected' ? [...selection.value] : entry ? [entry] : []
  name.value = kind === 'rename' ? entry?.name ?? '' : ''; modalError.value = ''
  showDialog(dialog.value); await nextTick(); input.value?.focus()
}
function close() { if (!busy.value) closeDialog(dialog.value) }
async function submit() {
  if (deleteAction.value && name.value !== confirmation.value) { modalError.value = 'Type the exact confirmation to continue.'; return }
  busy.value = true; modalError.value = ''
  try {
    if (action.value === 'create') await files.create(name.value)
    else if (action.value === 'rename' && targets.value[0]) await files.rename(targets.value[0],name.value)
    else {
      // Stop at the first error; successful deletions stay removed and remaining
      // targets are retained for a deliberate retry. Never report partial success as all done.
      while (targets.value.length) {
        await request('/files','DELETE',{path:targets.value[0]!.path})
        targets.value.shift()
      }
      await files.load()
    }
    busy.value = false; close()
  } catch (cause) { modalError.value = (cause as Error).message; if (deleteAction.value) await files.load() }
  finally { busy.value = false }
}
function menuAction(kind: 'open' | 'rename' | 'delete', entry: FileEntry) { if (kind === 'open') void files.load(entry.path); else void open(kind,entry) }
function fileKey(event: KeyboardEvent, entry: FileEntry) { if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) context(event,entry) }
function dragEnter(event: DragEvent) { if (!files.status?.permissions.upload || !event.dataTransfer?.types.includes('Files') || document.querySelector('dialog[open]')) return; dragDepth++; dragging.value = true }
function dragLeave() { if (--dragDepth <= 0) { dragDepth = 0; dragging.value = false } }
function drop(event: DragEvent) { dragging.value = false; dragDepth = 0; transfers.value?.drop(event) }
function modified(value: number | null) { return value === null ? '—' : new Date(value*1000).toLocaleString(undefined,{dateStyle:'medium',timeStyle:'short'}) }
watch(() => files.path, () => { actions.value?.close(); selected.value = []; search.value = ''; page.value = 1 })
watch(() => files.entries, entries => { selected.value = selected.value.filter(path => entries.some(entry => entry.path === path)) })
watch(search, () => { page.value = 1 })
watch(page, () => { fileArea.value?.scrollTo({top:0}) })
watch(pageCount, count => { page.value = Math.min(page.value, count) })
onMounted(() => { void files.start() })
onUnmounted(() => { files.stop(); closeDialog(dialog.value); closeDialog(downloads.value) })
</script>
<template>
  <main class="manager" @dragenter.prevent="dragEnter" @dragleave.prevent="dragLeave" @dragover.prevent @drop.prevent="drop">
    <header class="topbar">
      <img src="/logo.png" alt="SplitShare" class="logo" />
      <div class="identity"><h1>SplitShare</h1><p><span class="connection" :class="{ online: files.connected }" role="status">{{ files.connected ? 'Connected' : files.initialized ? 'Offline' : 'Connecting…' }}</span><span class="access-label"> · {{ files.status?.local_client ? 'Host access' : allPermissions ? 'Full access' : 'Shared folder' }}</span></p></div>
      <span v-if="files.status" class="mode-badge">{{ files.status.share_mode === 'open_lan' ? 'Open LAN' : 'Private link' }}</span>
      <div class="host-actions"><Sharing /></div>
    </header>
    <div class="workspace-tools">
      <nav class="breadcrumbs" aria-label="Breadcrumb">
        <button class="icon-btn parent-button" aria-label="Parent folder" :disabled="files.path === '/' || !files.status?.permissions.browse" @click="files.load(files.path.slice(0,files.path.lastIndexOf('/')) || '/')"><ArrowUp :size="17" /></button>
        <template v-for="(crumb,index) in crumbs" :key="crumb.path"><ChevronRight v-if="index" :size="14" aria-hidden="true" /><button :aria-current="index === crumbs.length-1 ? 'location' : undefined" @click="files.load(crumb.path)">{{ crumb.name }}</button></template>
      </nav>
      <section class="toolbar" aria-label="File controls">
        <label class="search"><Search :size="17" aria-hidden="true" /><input v-model="search" data-focus-fallback aria-label="Search files" placeholder="Search files…" /><button v-if="search" class="clear-search" aria-label="Clear search" @click="search = ''"><X :size="16" /></button></label>
        <Transfers ref="transfers" :path="files.path" :enabled="!!files.status?.permissions.upload" :concurrency="files.status?.upload_concurrency ?? 1" />
        <button class="button create-button" aria-label="Create folder" :disabled="!files.status?.permissions.create_directory" @click="open('create')"><FolderPlus :size="18" /><span class="desktop-label">New folder</span></button>
        <button class="icon-btn" aria-label="Refresh files" :disabled="files.refreshing" @click="files.refresh()"><RefreshCw :size="18" :class="{spinning:files.refreshing}" /></button>
        <div class="view-switch" aria-label="File view"><button class="icon-btn" aria-label="List view" :aria-pressed="!grid" @click="grid = false"><List :size="18" /></button><button class="icon-btn" aria-label="Grid view" :aria-pressed="grid" @click="grid = true"><LayoutGrid :size="18" /></button></div>
      </section>
    </div>
    <div v-if="selected.length" class="selection-bar" role="region" aria-label="Selected items"><span>{{ selected.length }} selected</span><button class="button" :disabled="!files.status?.permissions.download || !selection.some(entry => entry.kind === 'file')" @click="showDialog(downloads)"><Download :size="16" />Download selected</button><button class="button danger-text" :disabled="!files.status?.permissions.delete" @click="open('delete-selected')"><Trash2 :size="16" />Delete selected</button><button class="icon-btn" aria-label="Clear selection" @click="selected = []"><X :size="17" /></button></div>
    <div v-if="!files.connected && files.initialized && files.status?.sharing && !files.sessionRequired" class="connection-notice" role="status"><WifiOff :size="17" />Connection interrupted. Reconnecting to the host…</div>
    <div v-if="files.error" class="error-banner" role="alert"><ShieldAlert :size="18" /><span>{{ files.error }}</span><button class="button" :disabled="files.refreshing" @click="files.refresh()">Try again</button></div>
    <section ref="fileArea" class="file-area" :aria-busy="files.loading || !files.initialized" aria-label="Files">
      <div v-if="!files.initialized || (files.loading && !files.entries.length)" class="empty" role="status"><LoaderCircle class="spinning" :size="28" /><h2>Loading files…</h2><p>Reading the shared folder.</p></div>
      <div v-else-if="files.sessionRequired" class="empty"><ShieldAlert :size="36" /><h2>Share link required</h2><p>Open a current link or scan a QR code from the host to connect.</p></div>
      <div v-else-if="!files.status" class="empty"><WifiOff :size="36" /><h2>Host unavailable</h2><p>Keep this page open to reconnect, or try Refresh.</p></div>
      <div v-else-if="!files.status.sharing" class="empty"><FolderOpen :size="36" /><h2>No folder is being shared</h2><p>Select a folder when starting SplitShare.</p></div>
      <div v-else-if="!files.status.permissions.browse" class="empty"><ShieldAlert :size="36" /><h2>Browsing is disabled</h2><p>The host controls which file actions are available.</p></div>
      <div v-else-if="files.error" class="empty"><FolderOpen :size="36" /><h2>Folder unavailable</h2><p>Try again or choose a parent folder from the breadcrumb.</p></div>
      <div v-else-if="!visible.length" class="empty"><Search v-if="search" :size="36" /><FolderOpen v-else :size="36" /><h2>{{ search ? 'No matching files' : 'No files to show' }}</h2><p>{{ search ? 'Try a different name or clear your search.' : 'Files added to this folder will appear here.' }}</p><button v-if="search" class="button" @click="search = ''">Clear search</button></div>
      <div v-else class="file-results">
        <div v-if="!grid" class="list-heading" role="presentation"><input type="checkbox" aria-label="Select all visible items" :checked="allSelected" :indeterminate="pageEntries.some(entry => selected.includes(entry.path)) && !allSelected" @change="toggleAll" /><span>Name</span><span>Size</span><span>Modified</span><span class="actions-heading">Actions</span></div>
        <div :class="grid ? 'file-grid' : 'file-list'" role="list" aria-label="Folder contents">
        <article v-for="entry in pageEntries" :key="entry.path" class="file-item" :class="{selected:selected.includes(entry.path)}" role="listitem" tabindex="0" :aria-label="entry.name" @contextmenu="context($event,entry)" @keydown="fileKey($event,entry)">
          <input v-model="selected" class="item-select" type="checkbox" :value="entry.path" :aria-label="`Select ${entry.name}`" />
          <FileIcon :name="entry.name" :kind="entry.kind" />
          <div class="file-detail"><button v-if="entry.kind === 'directory'" class="file-name" :title="entry.name" @click="files.load(entry.path)">{{ entry.name }}</button><span v-else class="file-name" :title="entry.name">{{ entry.name }}</span><small>{{ typeLabels[fileCategory(entry.name,entry.kind)] }}<span v-if="entry.kind === 'file'" class="mobile-meta"> · {{ formatBytes(entry.size) }}</span></small></div>
          <span class="file-size">{{ entry.kind === 'directory' ? '—' : formatBytes(entry.size) }}</span><time class="modified" :datetime="entry.modified_unix_seconds === null ? undefined : new Date(entry.modified_unix_seconds*1000).toISOString()">{{ modified(entry.modified_unix_seconds) }}</time>
          <div class="file-actions"><a v-if="entry.kind === 'file' && files.status?.permissions.download" :href="downloadUrl(entry.path)" download class="icon-btn download-action" :aria-label="`Download ${entry.name}`"><Download :size="18" /></a><button class="icon-btn" :aria-label="`Actions for ${entry.name}`" @click="context($event,entry)"><MoreHorizontal :size="19" /></button></div>
        </article>
        </div>
      </div>
    </section>
    <nav v-if="pageCount > 1 && !files.error" class="pagination" aria-label="File pages">
      <button class="button" :disabled="page === 1" @click="page--">Previous page</button>
      <span aria-live="polite">{{ page }} / {{ pageCount }}</span>
      <button class="button" :disabled="page === pageCount" @click="page++">Next page</button>
    </nav>
    <div id="transfer-footer"></div>
    <footer class="statusbar"><span>{{ search ? `${visible.length} of ${files.entries.length}` : files.entries.length }} items</span><span>{{ files.status?.root_label ?? 'Shared folder' }} · Trusted private network</span></footer>
    <div v-if="dragging" class="drop-overlay" aria-hidden="true"><FolderPlus :size="42" /><strong>Drop files to upload</strong><span>{{ files.path }}</span></div>
    <FileActions ref="actions" :permissions="files.status?.permissions" @action="menuAction" />
    <dialog ref="dialog" :aria-label="title" @cancel.prevent="close" @click="($event.target === dialog) && close()">
      <form @submit.prevent="submit"><h2>{{ title }}</h2><p v-if="deleteAction">Permanently delete {{ action === 'delete-selected' ? `${targets.length} selected items` : `“${targets[0]?.name}”` }}? Folders must be empty. Type {{ confirmation }} to confirm.</p><p v-else>{{ action === 'create' ? 'Create a folder in the current directory.' : 'Choose a new name for this item.' }}</p>
        <label for="item-name">{{ deleteAction ? 'Confirm deletion' : 'Name' }}</label><input id="item-name" ref="input" v-model="name" required :disabled="busy" autocomplete="off" />
        <p v-if="modalError" role="alert" class="error">{{ modalError }}</p><div class="dialog-actions"><button type="button" class="button" :disabled="busy" @click="close">Cancel</button><button class="button" :class="deleteAction ? 'danger' : 'primary'" :disabled="busy">{{ busy ? 'Saving…' : deleteAction ? 'Delete' : 'Save' }}</button></div>
      </form>
    </dialog>
    <dialog ref="downloads" aria-label="Selected downloads" @cancel.prevent="closeDialog(downloads)"><h2>Download selected files</h2><p>Choose a file to download. Folders are not packaged into an archive.</p><div class="selected-downloads"><a v-for="entry in selection.filter(entry => entry.kind === 'file')" :key="entry.path" :href="downloadUrl(entry.path)" download class="button"><Download :size="17" />{{ entry.name }}</a></div><div class="dialog-actions"><button class="button primary" @click="closeDialog(downloads)">Done</button></div></dialog>
  </main>
</template>
