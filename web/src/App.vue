<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { Folder, File, Download, LayoutGrid, List, FolderPlus, RefreshCw, ChevronRight, ArrowUp, Search, MoreHorizontal, Pencil, Trash2, X } from '@lucide/vue'
import Transfers from './components/Transfers.vue'
import { useFiles } from './app/files'
import { downloadUrl, type FileEntry } from './app/api'
const files = useFiles()
const transfers = ref<InstanceType<typeof Transfers>>()
const grid = ref(false)
const search = ref('')
const visible = computed(() => files.entries.filter(entry => entry.name.toLocaleLowerCase().includes(search.value.toLocaleLowerCase())))
const crumbs = computed(() => [{ name: 'Shared folder', path: '/' }, ...files.path.split('/').filter(Boolean).map((name, index, parts) => ({ name, path: '/' + parts.slice(0,index + 1).join('/') }))])
const dialog = ref<HTMLDialogElement>()
const input = ref<HTMLInputElement>()
const action = ref<'create' | 'rename' | 'delete'>('create')
const target = ref<FileEntry>()
const name = ref('')
const modalError = ref('')
const busy = ref(false)
const menu = ref<{ entry: FileEntry; x: number; y: number }>()
const title = computed(() => ({ create: 'Create folder', rename: 'Rename item', delete: 'Delete item' })[action.value])
let focusBefore: HTMLElement | null = null
function context(event: MouseEvent, entry: FileEntry) {
  event.preventDefault()
  menu.value = { entry, x: Math.min(event.clientX, innerWidth - 210), y: Math.min(event.clientY, innerHeight - 180) }
}
async function open(kind: typeof action.value, entry?: FileEntry) {
  focusBefore = document.activeElement as HTMLElement
  menu.value = undefined
  action.value = kind; target.value = entry; name.value = kind === 'rename' ? entry?.name ?? '' : ''; modalError.value = ''
  dialog.value?.showModal()
  await nextTick(); input.value?.focus()
}
function close() { if (!busy.value) { dialog.value?.close(); focusBefore?.focus() } }
async function submit() {
  if (action.value === 'delete' && name.value !== target.value?.name) { modalError.value = 'Type the exact item name to confirm.'; return }
  busy.value = true; modalError.value = ''
  try {
    if (action.value === 'create') await files.create(name.value)
    else if (target.value && action.value === 'rename') await files.rename(target.value,name.value)
    else if (target.value) await files.remove(target.value)
    busy.value = false; close()
  } catch (error) { modalError.value = (error as Error).message }
  finally { busy.value = false }
}
function dismiss(event: KeyboardEvent) { if (event.key === 'Escape') menu.value = undefined }
function size(bytes: number | null) { if (bytes === null) return 'Folder'; if (bytes < 1024) return `${bytes} B`; if (bytes < 1024 ** 2) return `${(bytes/1024).toFixed(1)} KiB`; return `${(bytes/1024**2).toFixed(1)} MiB` }
onMounted(() => { void files.start(); window.addEventListener('keydown', dismiss) })
onUnmounted(() => { files.stop(); window.removeEventListener('keydown', dismiss) })
</script>

<template>
  <main class="manager" @click="menu = undefined" @dragover.prevent @drop.prevent="transfers?.drop($event)">
    <header class="topbar">
      <img src="/logo.png" alt="SplitShare" class="logo" />
      <div><h1>SplitShare</h1><p>{{ files.status?.share_mode === 'open_lan' ? 'Open LAN · Trusted private network' : 'Local access' }}</p></div>
      <span class="connection" :class="{ online: files.connected }">{{ files.connected ? 'Connected' : 'Offline' }}</span>
    </header>
    <nav class="breadcrumbs" aria-label="Breadcrumb">
      <template v-for="(crumb, index) in crumbs" :key="crumb.path"><ChevronRight v-if="index" :size="15" /><button @click="files.load(crumb.path)">{{ crumb.name }}</button></template>
    </nav>
    <section class="toolbar" aria-label="File controls">
      <button class="icon-btn" aria-label="Parent folder" :disabled="files.path === '/'" @click="files.load(files.path.slice(0,files.path.lastIndexOf('/')) || '/')"><ArrowUp :size="18" /></button>
      <label class="search"><Search :size="17" /><input v-model="search" aria-label="Search files" placeholder="Search files…" /></label>
      <button class="button primary" :disabled="!files.status?.permissions.create_directory" @click="open('create')"><FolderPlus :size="18" /><span class="desktop-label">New folder</span><span class="sr-only">Create folder</span></button>
      <button class="icon-btn" aria-label="Refresh files" :disabled="!files.status?.sharing || files.loading" @click="files.load()"><RefreshCw :size="18" /></button>
      <Transfers ref="transfers" :path="files.path" :enabled="!!files.status?.permissions.upload" :concurrency="files.status?.upload_concurrency ?? 1" />
      <div class="view-switch"><button class="icon-btn" aria-label="List view" :aria-pressed="!grid" @click="grid = false"><List :size="18" /></button><button class="icon-btn" aria-label="Grid view" :aria-pressed="grid" @click="grid = true"><LayoutGrid :size="18" /></button></div>
    </section>
    <p v-if="files.error" class="error" role="alert">{{ files.error }}</p>
    <section class="file-area" :aria-busy="files.loading" aria-label="Files">
      <div v-if="files.status && !files.status.sharing" class="empty">No folder is being shared. Select a folder when starting SplitShare.</div>
      <div v-else-if="!visible.length" class="empty">{{ files.loading ? 'Loading files…' : 'No files to show' }}</div>
      <div v-else :class="grid ? 'file-grid' : 'file-list'">
        <article v-for="entry in visible" :key="entry.path" class="file-item" @contextmenu="context($event, entry)">
          <component :is="entry.kind === 'directory' ? Folder : File" :size="grid ? 36 : 24" class="file-icon" />
          <div class="file-detail"><button v-if="entry.kind === 'directory'" class="file-name" @click="files.load(entry.path)">{{ entry.name }}</button><span v-else class="file-name">{{ entry.name }}</span><small>{{ size(entry.size) }}</small></div>
          <time class="modified">{{ entry.modified_unix_seconds ? new Date(entry.modified_unix_seconds * 1000).toLocaleDateString() : '—' }}</time>
          <a v-if="entry.kind === 'file'" :href="downloadUrl(entry.path)" class="icon-btn" :aria-label="`Download ${entry.name}`"><Download :size="18" /></a>
          <button class="icon-btn" :aria-label="`Actions for ${entry.name}`" @click.stop="context($event,entry)"><MoreHorizontal :size="18" /></button>
        </article>
      </div>
    </section>
    <footer>{{ files.entries.length }} items <span>{{ files.status?.root_label ?? 'Shared folder' }}</span></footer>
    <div v-if="menu" class="context-menu" role="group" aria-label="Item actions" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" @click.stop>
      <strong>{{ menu.entry.name }}</strong>
      <button @click="open('rename',menu.entry)"><Pencil :size="17" />Rename</button>
      <button class="danger-text" @click="open('delete',menu.entry)"><Trash2 :size="17" />Delete</button>
      <button @click="menu = undefined"><X :size="17" />Close</button>
    </div>
    <dialog ref="dialog" @cancel.prevent="close()" @click="($event.target === dialog) && close()">
      <form @submit.prevent="submit">
        <h2>{{ title }}</h2>
        <p v-if="action === 'delete'">Permanently delete “{{ target?.name }}”? Type its name to confirm. Folders must be empty.</p>
        <label :for="'item-name'">{{ action === 'delete' ? 'Confirm item name' : 'Name' }}</label>
        <input id="item-name" ref="input" v-model="name" required :disabled="busy" autocomplete="off" />
        <p v-if="modalError" role="alert" class="error">{{ modalError }}</p>
        <div class="dialog-actions"><button type="button" class="button" :disabled="busy" @click="close">Cancel</button><button class="button" :class="action === 'delete' ? 'danger' : 'primary'" :disabled="busy">{{ busy ? 'Saving…' : action === 'delete' ? 'Delete' : 'Save' }}</button></div>
      </form>
    </dialog>
  </main>
</template>
