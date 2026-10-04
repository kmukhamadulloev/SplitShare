<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { Download, X, ZoomIn, ZoomOut } from '@lucide/vue'
import { downloadUrl, type FileEntry } from '../app/api'
import { showDialog, closeDialog } from '../app/dialogs'
const props = defineProps<{ allowed: boolean; entries: FileEntry[] }>()
const dialog = ref<HTMLDialogElement>()
const entry = ref<FileEntry>()
const kind = ref(''), url = ref(''), text = ref(''), error = ref('')
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
  <dialog ref="dialog" class="preview-dialog" aria-label="File preview" @cancel.prevent="close" @keydown="keyboard">
    <header class="preview-header"><h2>{{ entry?.name }}</h2><button class="icon-btn" aria-label="Close preview" @click="close"><X :size="20" /></button></header>
    <p v-if="loading" role="status">Loading preview…</p>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <p v-if="truncated" role="status">Showing the first 256 KiB. Download to read the complete file.</p>
    <div class="preview-body" tabindex="0" aria-label="Preview content">
      <img v-if="kind === 'image' && !error" :src="url" :alt="entry?.name" :style="{width:zoom === 1 ? undefined : `${zoom*100}%`,maxWidth:zoom === 1 ? '100%' : 'none',maxHeight:zoom === 1 ? '100%' : 'none'}" @load="loading = false" @error="failed" />
      <video v-if="kind === 'video' && !error" :src="url" controls playsinline preload="metadata" tabindex="0" aria-label="Video player" @loadedmetadata="loading = false" @error="failed"></video>
      <audio v-if="kind === 'audio' && !error" :src="url" controls preload="metadata" tabindex="0" aria-label="Audio player" @loadedmetadata="loading = false" @error="failed"></audio>
      <pre v-if="kind === 'text'" :class="{wrap}">{{ text || 'Empty file.' }}</pre>
    </div>
    <footer class="dialog-actions">
      <template v-if="kind === 'image' && !error"><button class="button" :disabled="index <= 0" @click="navigate(-1)">Previous image</button><button class="button" :disabled="index < 0 || index >= images.length-1" @click="navigate(1)">Next image</button><button class="icon-btn" aria-label="Zoom out" :disabled="zoom <= 1" @click="zoom = Math.max(1,zoom-.5)"><ZoomOut :size="18" /></button><button class="icon-btn" aria-label="Zoom in" :disabled="zoom >= 3" @click="zoom += .5"><ZoomIn :size="18" /></button></template>
      <button v-if="kind === 'text'" class="button" :aria-pressed="wrap" @click="wrap = !wrap">Wrap text</button>
      <button v-if="error && entry" class="button" @click="open(entry)">Retry preview</button>
      <a v-if="entry && allowed" class="button primary" :href="downloadUrl(entry.path)" download><Download :size="18" />Download</a>
    </footer>
  </dialog>
</template>
