<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { Download, X, ZoomIn, ZoomOut, ChevronLeft, ChevronRight, Maximize, Music2, LoaderCircle, FileWarning, WrapText, Minus, Plus } from '@lucide/vue'
import { formatBytes } from '../app/file-types'
import { downloadUrl, type FileEntry } from '../app/api'
import { showDialog, closeDialog } from '../app/dialogs'
const props = defineProps<{ allowed: boolean; entries: FileEntry[] }>()
const dialog = ref<HTMLDialogElement>()
const entry = ref<FileEntry>()
const kind = ref(''), url = ref(''), text = ref(''), error = ref('')
const fontSize = ref(14)
const loading = ref(false), truncated = ref(false), wrap = ref(true), zoom = ref(1)
const images = computed(() => props.entries.filter(file => file.kind === 'file' && /\.(png|jpe?g|gif|webp|avif)$/i.test(file.name)))
const index = computed(() => images.value.findIndex(file => file.path === entry.value?.path))
function navigate(offset: number) { const file = images.value[index.value+offset]; if (file) void open(file) }
function keyboard(event: KeyboardEvent) {
  if (kind.value !== 'image' || !['ArrowLeft','ArrowRight'].includes(event.key)) return
  event.preventDefault(); navigate(event.key === 'ArrowLeft' ? -1 : 1)
}
let controller: AbortController | undefined
let generation = 0
const limit = 256 * 1024
function stop() {
  generation++; controller?.abort()
  dialog.value?.querySelectorAll('video,audio').forEach(element => {
    const media = element as HTMLMediaElement
    media.pause(); media.removeAttribute('src'); media.load()
  })
  url.value = ''; kind.value = ''; text.value = ''
}
function close() { stop(); closeDialog(dialog.value); entry.value = undefined }
function failed() { loading.value = false; error.value = 'This file could not be previewed. It may be unavailable, damaged, or unsupported by this browser.' }
async function open(file: FileEntry) {
  if (!props.allowed) return
  stop(); const current = generation
  entry.value = file; error.value = ''; loading.value = true; truncated.value = false; zoom.value = 1
  await nextTick()
  if (current !== generation || !props.allowed) return
  showDialog(dialog.value)
  controller = new AbortController()
  const signal = controller.signal
  const source = `/api/v1/files/preview?${new URLSearchParams({path:file.path})}`
  try {
    const head = await fetch(source,{method:'HEAD',signal})
    if (!head.ok) throw new Error(head.status === 415 ? 'Preview unavailable for this file type.' : 'File unavailable or preview access denied.')
    const mime = head.headers.get('content-type') ?? ''
    if (current !== generation) return
    if (mime.startsWith('text/plain')) {
      const size = Number(head.headers.get('content-length'))
      if (size > 0) {
        const response = await fetch(source,{signal,headers:{Range:`bytes=0-${limit-1}`}})
        if (!response.ok || !response.body) throw new Error('Text preview could not be loaded.')
        const reader = response.body.getReader(), decoder = new TextDecoder('utf-8',{fatal:true})
        let bytes = 0, content = ''
        try {
          while (bytes < limit) {
            const part = await reader.read()
            if (part.done) break
            const chunk = part.value.subarray(0,limit-bytes)
            content += decoder.decode(chunk,{stream:true}); bytes += chunk.length
          }
          if (size <= limit) content += decoder.decode()
        } finally { await reader.cancel() }
        if (current !== generation) return
        if (content.includes('\0')) throw new Error('This file is not supported UTF-8 text.')
        text.value = content; truncated.value = size > limit
      }
      kind.value = 'text'; loading.value = false
    } else if (/^(image|video|audio)\//.test(mime)) {
      kind.value = mime.split('/')[0]!; url.value = source
    } else throw new Error('Preview unavailable for this file type.')
  } catch (cause) {
    if (current !== generation) return
    loading.value = false
    error.value = cause instanceof TypeError ? 'Unable to load preview. Check the connection and file encoding.' : (cause as Error).message
  }
}
watch(() => props.allowed, allowed => { if (!allowed) close() })
onUnmounted(close)
defineExpose({open,close})
</script>
<template>
  <dialog ref="dialog" class="preview-dialog" :class="{'preview-audio':kind === 'audio'}" aria-label="File preview" @cancel.prevent="close" @keydown="keyboard">
    <header class="preview-header">
      <div class="preview-title"><h2 :title="entry?.name">{{ entry?.name }}</h2><p>{{ kind ? ({image:'Image',video:'Video',audio:'Audio',text:'Text'}[kind]) : 'File preview' }} · {{ formatBytes(entry?.size ?? null) }}<span v-if="kind === 'text'"> · Read only</span></p></div>
      <a v-if="entry && allowed" class="icon-btn" :href="downloadUrl(entry.path)" download aria-label="Download" title="Download file"><Download :size="20" /></a>
      <button class="icon-btn" aria-label="Close preview" title="Close (Esc)" @click="close"><X :size="20" /></button>
    </header>
    <p v-if="truncated" class="preview-notice" role="status">Showing the first 256 KiB. Download to read the complete file.</p>
    <div class="preview-stage">
      <div v-if="loading" class="preview-state" role="status"><LoaderCircle class="spinning" :size="28" /><span>Loading preview…</span></div>
      <div v-if="error" class="preview-state preview-error" role="alert"><FileWarning :size="36" /><strong>Preview unavailable</strong><p>{{ error }}</p><button v-if="entry" class="button" @click="open(entry)">Retry preview</button><a v-if="entry && allowed" class="button primary" :href="downloadUrl(entry.path)" download>Download file</a></div>
      <div class="preview-body" :class="{'preview-image-body':kind === 'image' && zoom === 1,'preview-audio-body':kind === 'audio'}" tabindex="0" aria-label="Preview content" :aria-busy="loading" v-show="!error">
        <img v-if="kind === 'image'" :src="url" :alt="entry?.name" :style="{width:zoom === 1 ? undefined : `${zoom*100}%`,maxWidth:zoom === 1 ? '100%' : 'none',maxHeight:zoom === 1 ? '100%' : 'none'}" @load="loading = false" @error="failed" />
        <video v-if="kind === 'video'" :src="url" controls playsinline preload="metadata" tabindex="0" aria-label="Video player" @loadedmetadata="loading = false" @error="failed"></video>
        <template v-if="kind === 'audio'"><div class="audio-art" aria-hidden="true"><Music2 :size="64" /></div><p class="audio-name">{{ entry?.name }}</p><audio :src="url" controls preload="metadata" tabindex="0" aria-label="Audio player" @loadedmetadata="loading = false" @error="failed"></audio></template>
        <pre v-if="kind === 'text'" :class="{wrap}" :style="{fontSize:`${fontSize}px`}">{{ text || 'Empty file.' }}</pre>
      </div>
    </div>
    <footer v-if="!error && (kind === 'image' || kind === 'text')" class="preview-tools">
      <template v-if="kind === 'image'">
        <div class="preview-tool-group"><button class="icon-btn" aria-label="Previous image" title="Previous image (←)" :disabled="index <= 0" @click="navigate(-1)"><ChevronLeft :size="20" /></button><span class="preview-count">{{ index + 1 }} / {{ images.length }}</span><button class="icon-btn" aria-label="Next image" title="Next image (→)" :disabled="index < 0 || index >= images.length-1" @click="navigate(1)"><ChevronRight :size="20" /></button></div>
        <div class="preview-tool-group"><button class="icon-btn" aria-label="Zoom out" title="Zoom out" :disabled="zoom <= 1" @click="zoom = Math.max(1,zoom-.5)"><ZoomOut :size="18" /></button><button class="button preview-fit" aria-label="Fit image" title="Reset to fit" @click="zoom = 1"><Maximize :size="16" />{{ zoom === 1 ? 'Fit' : `${zoom}×` }}</button><button class="icon-btn" aria-label="Zoom in" title="Zoom in" :disabled="zoom >= 3" @click="zoom += .5"><ZoomIn :size="18" /></button></div>
      </template>
      <template v-if="kind === 'text'"><button class="button" :aria-pressed="wrap" @click="wrap = !wrap"><WrapText :size="18" />Wrap text</button><div class="preview-tool-group"><button class="icon-btn" aria-label="Smaller text" title="Smaller text" :disabled="fontSize <= 12" @click="fontSize -= 2"><Minus :size="18" /></button><span>{{ fontSize }} px</span><button class="icon-btn" aria-label="Larger text" title="Larger text" :disabled="fontSize >= 24" @click="fontSize += 2"><Plus :size="18" /></button></div></template>
    </footer>
  </dialog>
</template>
