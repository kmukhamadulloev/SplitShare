<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Upload, ClipboardPaste, X, RotateCcw, ListChecks } from '@lucide/vue'
import { useUploads, type QueueItem } from '../app/uploads'
import { showDialog, closeDialog } from '../app/dialogs'
import { formatBytes } from '../app/file-types'
const props = defineProps<{ path: string; enabled: boolean; concurrency: number }>()
const uploads = useUploads()
watch(() => props.enabled, value => uploads.setEnabled(value), { immediate: true })
const picker = ref<HTMLInputElement>()
const queue = ref<HTMLDialogElement>()
const pasteDialog = ref<HTMLDialogElement>()
const conflictDialog = ref<HTMLDialogElement>()
const filename = ref('clipboard.txt')
const draft = ref('')
const preview = ref('')
const imageFile = ref<File>()
const pasteError = ref('')
const awaitingClipboard = ref(false)
const readingClipboard = ref(false)
const pasteTitle = computed(() => awaitingClipboard.value ? 'Paste from clipboard' : imageFile.value ? 'Paste image' : 'Save clipboard text')
const nameInput = ref<HTMLInputElement>()
const conflicts = computed(() => uploads.items.filter(item => item.state === 'failed' && item.failure === 'CONFLICT'))
const conflict = ref<QueueItem>()
const queuedImages: File[] = []
const show = showDialog
const close = closeDialog
function bytes(value: number | null) { return value === null ? 'Unknown' : formatBytes(value) }
function select(event: Event) { const input = event.target as HTMLInputElement; uploads.enqueue([...input.files ?? []],props.path); input.value = '' }
function drop(event: DragEvent) { if (props.enabled && event.dataTransfer?.files.length) { event.preventDefault(); uploads.enqueue([...event.dataTransfer.files],props.path) } }
async function prepareText(text: string) {
  if (text.length > 2 * 1024 * 1024) { uploads.problem = 'Clipboard text is limited to 2 MiB.'; return }
  awaitingClipboard.value = false
  imageFile.value = undefined; revoke(); draft.value = text; filename.value = 'clipboard.txt'; pasteError.value = ''
  try { JSON.parse(text); filename.value = 'clipboard.json' } catch { /* Plain text remains plain text. */ }
  show(pasteDialog.value); await nextTick(); nameInput.value?.focus()
}
function revoke() { if (preview.value) URL.revokeObjectURL(preview.value); preview.value = '' }
function nextImage() {
  const file = queuedImages.shift(); if (!file) return
  awaitingClipboard.value = false
  imageFile.value = file; draft.value = ''; filename.value = file.name || 'clipboard.png'; pasteError.value = ''; revoke(); preview.value = URL.createObjectURL(file)
  show(pasteDialog.value)
}
function clipboardFiles(files: File[]) {
  const images = files.filter(file => /^image\/(png|jpeg|webp|gif)$/.test(file.type))
  uploads.enqueue(files.filter(file => !images.includes(file)),props.path)
  queuedImages.push(...images)
  if (!pasteDialog.value?.open) nextImage()
}
function paste(event: ClipboardEvent) {
  if (!props.enabled) return
  const inPasteDialog = !!pasteDialog.value?.open
  if (!inPasteDialog && (document.querySelector('dialog[open]') || (event.target as HTMLElement).closest('input, textarea, [contenteditable=true]'))) return
  const data = event.clipboardData
  if (!data) return
  // Some browsers expose clipboard files only through DataTransferItemList.
  const items = Array.from(data.items ?? []).filter(item => item.kind === 'file')
  const files = Array.from(data.files)
  if (!files.length) files.push(...items.map(item => item.getAsFile()).filter((file): file is File => file !== null))
  if (files.length) {
    event.preventDefault()
    if (inPasteDialog) { close(pasteDialog.value); revoke(); imageFile.value = undefined }
    clipboardFiles(files)
    return
  }
  if (items.length) { event.preventDefault(); uploads.problem = 'The browser could not read the clipboard file. Try copying the image again or use Upload files.'; return }
  // Keep normal text editing inside the fallback dialog.
  if (inPasteDialog && !awaitingClipboard.value) return
  const text = data.getData('text/plain')
  if (text) { event.preventDefault(); void prepareText(text) }
}
function awaitClipboard(message: string) {
  awaitingClipboard.value = true; imageFile.value = undefined; revoke(); draft.value = ''; pasteError.value = message
  show(pasteDialog.value)
}
async function readClipboard() {
  if (readingClipboard.value || !props.enabled) return
  if (!navigator.clipboard?.read) {
    awaitClipboard(window.isSecureContext
      ? 'This browser does not support reading images with the Paste button. Press Ctrl+V (⌘V on Mac) here or use Upload files.'
      : 'This HTTP address does not allow clipboard access from the Paste button. On the host, open SplitShare through localhost. Otherwise press Ctrl+V (⌘V on Mac) here or use Upload files.')
    return
  }
  readingClipboard.value = true
  try {
    const items = await navigator.clipboard.read()
    const images: File[] = []
    // Inspect the whole clipboard before considering text representations.
    for (const item of items) {
      const image = item.types.find(type => /^image\/(png|jpeg|webp|gif)$/.test(type))
      if (image) images.push(new File([await item.getType(image)],`clipboard.${image.split('/')[1]}`,{ type: image }))
    }
    if (images.length) { if (awaitingClipboard.value) close(pasteDialog.value); awaitingClipboard.value = false; clipboardFiles(images); return }
    const text = items.find(item => item.types.includes('text/plain'))
    if (text) { await prepareText(await (await text.getType('text/plain')).text()); return }
    awaitClipboard('No supported image or text was exposed by the browser. Copy the image itself, then retry, or use Upload files.')
  } catch (cause) {
    awaitClipboard(cause instanceof DOMException && cause.name === 'NotAllowedError'
      ? 'Clipboard permission was denied. Allow clipboard access in your browser and retry, or press Ctrl+V (⌘V on Mac) here.'
      : 'The browser could not read the clipboard. Try copying the image again, then retry, or press Ctrl+V (⌘V on Mac) here.')
  } finally { readingClipboard.value = false }
}
function savePaste() {
  if (awaitingClipboard.value) return
  if (!filename.value.trim() || filename.value.includes('/') || filename.value.includes('\\')) { pasteError.value = 'Enter a filename without folders.'; return }
  if (!imageFile.value && new Blob([draft.value]).size > 2 * 1024 * 1024) { pasteError.value = 'Clipboard text is limited to 2 MiB.'; return }
  const file = new File([imageFile.value ?? draft.value],filename.value,{ type: imageFile.value?.type ?? 'text/plain' })
  uploads.enqueue([file],props.path); dismissPaste()
}
function dismissPaste() { close(pasteDialog.value); revoke(); imageFile.value = undefined; nextImage() }
function resolve(policy?: QueueItem['policy'] | 'keep') { if (conflict.value && policy === 'keep') uploads.keepExisting(conflict.value); else if (conflict.value && policy && policy !== 'keep') uploads.retry(conflict.value,policy); close(conflictDialog.value); conflict.value = undefined }
function showConflict(item: QueueItem) { conflict.value = item; show(conflictDialog.value) }
watch(() => [props.enabled,props.concurrency], () => { if (props.enabled) uploads.start(props.concurrency) }, { immediate: true })
onMounted(() => document.addEventListener('paste',paste))
onUnmounted(() => { document.removeEventListener('paste',paste); uploads.stop(); revoke(); close(queue.value); close(pasteDialog.value); close(conflictDialog.value) })
defineExpose({ drop })
</script>
<template>
  <input ref="picker" type="file" multiple hidden aria-label="Upload files" @change="select" />
  <button class="button paste-button" :disabled="!enabled || readingClipboard" aria-label="Paste" @click="readClipboard"><ClipboardPaste :size="18" /><span class="desktop-label">Paste</span></button>
  <button class="icon-btn primary upload-button" :disabled="!enabled" aria-label="Upload files" @click="picker?.click()"><Upload :size="18" /></button>
  <Teleport defer to="#transfer-footer">
  <section v-if="uploads.items.length" class="transfer-summary" aria-label="Overall transfer progress">
    <button class="summary-open" aria-label="Open upload queue" @click="show(queue)"><ListChecks :size="20" /><span>{{ uploads.items.length }} files · {{ uploads.progress.active }} active · {{ uploads.progress.queued }} queued<span v-if="uploads.items.some(item => item.state === 'failed')"> · {{ uploads.items.filter(item => item.state === 'failed').length }} failed</span><span v-if="uploads.items.some(item => item.state === 'cancelled')"> · {{ uploads.items.filter(item => item.state === 'cancelled').length }} cancelled</span></span><strong>{{ uploads.progress.percent === null ? 'Unknown total' : `${uploads.progress.percent.toFixed(0)}%` }}</strong></button>
    <progress v-if="uploads.progress.percent !== null" :value="uploads.progress.percent" max="100" aria-label="Aggregate upload progress"></progress>
    <div class="summary-meta">{{ bytes(uploads.progress.transferred) }} / {{ bytes(uploads.progress.total) }}<span>{{ bytes(uploads.speed) }}/s<span v-if="uploads.eta !== null"> · {{ uploads.eta }}s remaining</span></span></div>
  </section>
  </Teleport>
  <p v-if="uploads.problem" class="transfer-problem" role="alert">{{ uploads.problem }} <button aria-label="Dismiss transfer message" @click="uploads.problem = ''">×</button></p>
  <dialog ref="queue" class="queue-dialog" aria-label="Upload queue" @cancel.prevent="close(queue)">
    <h2>Upload queue</h2><p>Parallel upload limit: {{ uploads.limit }}</p>
    <p v-if="!uploads.items.length" class="queue-empty">No transfers in the queue.</p>
    <div v-for="item in uploads.items" :key="item.id" class="queue-item">
      <div><strong>{{ item.file.name }}</strong><span class="transfer-state">{{ item.state }}</span></div>
      <progress v-if="item.total !== null && item.total > 0" :value="item.transferred" :max="item.total" :aria-label="`Upload progress for ${item.file.name}`"></progress>
      <small>{{ bytes(item.transferred) }} / {{ bytes(item.total) }} <span v-if="item.failure">· {{ item.failure }}</span></small>
      <button v-if="['queued','uploading'].includes(item.state)" class="button" :aria-label="`Cancel ${item.file.name}`" @click="uploads.cancel(item)"><X :size="16" />Cancel</button>
      <button v-if="item.state === 'failed' && item.failure === 'CONFLICT'" class="button" :disabled="!enabled || !!item.controller" @click="showConflict(item)">Resolve conflict</button>
      <p v-if="item.state === 'unconfirmed'">The host has not confirmed the outcome. Check the destination before retrying; existing files will require conflict resolution.</p>
      <button v-if="item.state === 'unconfirmed'" class="button" @click="uploads.refresh()">Check status</button>
      <button v-if="['failed','cancelled','unconfirmed'].includes(item.state) && item.failure !== 'CONFLICT'" class="button" :disabled="!enabled || !!item.controller" @click="uploads.retry(item)"><RotateCcw :size="16" />Retry</button>
    </div>
    <div class="dialog-actions"><button class="button" @click="uploads.clear()">Clear finished</button><button class="button primary" @click="close(queue)">Close queue</button></div>
  </dialog>
  <dialog ref="pasteDialog" :aria-label="pasteTitle" @cancel.prevent="dismissPaste">
    <form @submit.prevent="savePaste"><h2>{{ pasteTitle }}</h2>
      <p v-if="awaitingClipboard">Clipboard contents have not been detected. Paste here to continue.</p>
      <img v-else-if="imageFile" :src="preview" alt="Pasted image preview" class="paste-preview" />
      <template v-else><label for="clipboard-text">Text</label><textarea id="clipboard-text" v-model="draft" rows="7" maxlength="2097152"></textarea></template>
      <template v-if="!awaitingClipboard"><label for="clipboard-name">Filename</label><input id="clipboard-name" ref="nameInput" v-model="filename" required /></template>
      <p v-if="pasteError" class="error" role="alert">{{ pasteError }}</p>
      <div class="dialog-actions"><button class="button" type="button" @click="dismissPaste">Cancel</button><button v-if="awaitingClipboard" class="button primary" type="button" :disabled="readingClipboard" @click="readClipboard">Retry clipboard</button><button v-else class="button primary" :disabled="!enabled">Save file</button></div>
    </form>
  </dialog>
  <dialog ref="conflictDialog" aria-label="File already exists" @cancel.prevent="resolve()"><h2>File already exists</h2><p>Choose what to do with “{{ conflict?.file.name }}”. Retrying uploads the file again from the beginning.</p><div class="dialog-actions"><button class="button" @click="resolve('keep')">Keep existing</button><button class="button" :disabled="!enabled" @click="resolve('auto_rename')">Save a copy</button><button class="button danger" :disabled="!enabled" @click="resolve('replace')">Replace</button></div></dialog>
  <button v-if="conflicts.length && !queue?.open" class="conflict-notice button" @click="show(queue)">{{ conflicts.length }} upload conflicts · Open queue</button>
</template>
