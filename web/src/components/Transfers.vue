<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { Upload, ClipboardPaste, X, RotateCcw, ListChecks, ImagePlus, FolderOpen } from '@lucide/vue'
import { useUploads, type QueueItem } from '../app/uploads'
import { showDialog, closeDialog } from '../app/dialogs'
import { formatBytes } from '../app/file-types'
const props = defineProps<{ path: string; enabled: boolean; concurrency: number }>()
const uploads = useUploads()
watch(() => props.enabled, value => uploads.setEnabled(value), { immediate: true })
const picker = ref<HTMLInputElement>()
const mediaPicker = ref<HTMLInputElement>()
const uploadSheet = ref<HTMLDialogElement>()
const previewFailed = ref(false)
const touchLayout = ref(false)
const touchQuery = matchMedia('(max-width: 767px), (pointer: coarse)')
function updateLayout() { touchLayout.value = touchQuery.matches }
function openUpload() {
  if (!props.enabled) return
  if (touchLayout.value) show(uploadSheet.value)
  else picker.value?.click()
}
function chooseFiles(media = false) {
  if (!props.enabled) return
  // Keep dialogs open until selection so cancelling the OS picker returns here.
  const target = media ? mediaPicker.value : picker.value
  target?.click()
}
function pasteFromSheet() { close(uploadSheet.value); void readClipboard() }
function selectFiles(event: Event) {
  const input = event.target as HTMLInputElement
  const files = [...input.files ?? []]; input.value = ''
  if (!props.enabled || !files.length) return
  close(uploadSheet.value)
  if (pasteDialog.value?.open) { close(pasteDialog.value); revoke(); imageFile.value = undefined }
  clipboardFiles(files)
}
function enteredPaste(event: Event) {
  const input = event.target as HTMLTextAreaElement
  if (props.enabled && awaitingClipboard.value && input.value) void prepareText(input.value)
}
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
const clipboardReadable = typeof navigator.clipboard?.read === 'function'
const pasteTitle = computed(() => awaitingClipboard.value ? 'Paste from clipboard' : imageFile.value ? 'Upload image' : 'Save clipboard text')
const nameInput = ref<HTMLInputElement>()
const conflicts = computed(() => uploads.items.filter(item => item.state === 'failed' && item.failure === 'CONFLICT'))
const conflict = ref<QueueItem>()
const queuedImages: File[] = []
const show = showDialog
const close = closeDialog
function bytes(value: number | null) { return value === null ? 'Unknown' : formatBytes(value) }
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
  imageFile.value = file; draft.value = ''; filename.value = file.name || 'clipboard.png'; previewFailed.value = false; pasteError.value = ''; revoke(); preview.value = URL.createObjectURL(file)
  show(pasteDialog.value)
}
function clipboardFiles(files: File[]) {
  const images = files.filter(file => file.type.startsWith('image/'))
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
  else if (awaitingClipboard.value) {
    event.preventDefault()
    pasteError.value = 'No image or text was provided. Choose a photo or browse files instead.'
  }
}
function awaitClipboard(message: string) {
  awaitingClipboard.value = true; imageFile.value = undefined; revoke(); draft.value = ''; pasteError.value = message
  show(pasteDialog.value)
}
async function readClipboard() {
  if (readingClipboard.value || !props.enabled) return
  if (!navigator.clipboard?.read) {
    awaitClipboard(window.isSecureContext
      ? 'Automatic clipboard reading is unavailable. Paste into the area below or choose a photo.'
      : 'This HTTP address does not support automatic clipboard reading. Paste into the area below or choose a photo.')
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
      ? 'Clipboard permission was denied. Paste into the area below, choose a photo, or allow access and retry.'
      : 'The browser could not read the clipboard. Paste into the area below, choose a photo, or retry.')
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
onMounted(() => { document.addEventListener('paste',paste); updateLayout(); touchQuery.addEventListener('change',updateLayout) })
onUnmounted(() => { document.removeEventListener('paste',paste); touchQuery.removeEventListener('change',updateLayout); close(uploadSheet.value); uploads.stop(); revoke(); close(queue.value); close(pasteDialog.value); close(conflictDialog.value) })
defineExpose({ drop })
</script>
<template>
  <input ref="picker" type="file" multiple hidden aria-label="Upload files" @change="selectFiles" />
  <input ref="mediaPicker" type="file" accept="image/*,video/*" multiple hidden aria-label="Choose photos and videos" @change="selectFiles" />
  <button class="button paste-button" :disabled="!enabled || readingClipboard" aria-label="Paste" @click="readClipboard"><ClipboardPaste :size="18" /><span class="desktop-label">Paste</span></button>
  <button class="button primary upload-button" :disabled="!enabled" aria-label="Upload files" @click="openUpload"><Upload :size="18" /><span>Upload</span></button>
  <dialog ref="uploadSheet" class="upload-sheet" aria-label="Upload" @cancel.prevent="close(uploadSheet)" @click="$event.target === uploadSheet && close(uploadSheet)">
    <div class="sheet-handle" aria-hidden="true"></div>
    <h2>Upload</h2><p>Add files to this shared folder.</p>
    <div class="upload-options">
      <button class="button" :disabled="!enabled" @click="chooseFiles(true)"><ImagePlus :size="22" /><span>Photos &amp; videos</span></button>
      <button class="button" :disabled="!enabled" @click="chooseFiles()"><FolderOpen :size="22" /><span>Browse files</span></button>
      <button class="button" :disabled="!enabled || readingClipboard" @click="pasteFromSheet"><ClipboardPaste :size="22" /><span>Paste from clipboard</span></button>
    </div>
    <div class="dialog-actions"><button class="button" @click="close(uploadSheet)">Cancel</button></div>
  </dialog>
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
  <dialog ref="pasteDialog" class="paste-dialog" :aria-label="pasteTitle" @cancel.prevent="dismissPaste">
    <form @submit.prevent="savePaste"><h2>{{ pasteTitle }}</h2>
      <template v-if="awaitingClipboard">
        <label for="paste-target">Paste area</label>
        <p id="paste-help">{{ touchLayout ? 'Touch and hold here, then choose Paste.' : 'Click here and press Ctrl+V (⌘V on Mac).' }}</p>
        <textarea id="paste-target" class="paste-target" rows="4" aria-describedby="paste-help" :placeholder="touchLayout ? 'Touch and hold to paste' : 'Paste an image, file or text'" spellcheck="false" @input="enteredPaste"></textarea>
        <div class="paste-alternatives"><button type="button" class="button" :disabled="!enabled" @click="chooseFiles(true)"><ImagePlus :size="18" />Choose photo</button><button type="button" class="button" :disabled="!enabled" @click="chooseFiles()"><FolderOpen :size="18" />Browse files</button></div>
      </template>
      <template v-else-if="imageFile">
        <img v-if="!previewFailed" :src="preview" alt="Image preview" class="paste-preview" @error="previewFailed = true" />
        <p v-else role="status">This image format cannot be previewed here. You can still upload the original file.</p>
        <p>{{ formatBytes(imageFile.size) }} · Uploads after confirmation.</p>
      </template>
      <template v-else><label for="clipboard-text">Text</label><textarea id="clipboard-text" v-model="draft" rows="7" maxlength="2097152"></textarea></template>
      <template v-if="!awaitingClipboard"><label for="clipboard-name">Filename</label><input id="clipboard-name" ref="nameInput" v-model="filename" required /></template>
      <p v-if="pasteError" class="error" role="alert">{{ pasteError }}</p>
      <div class="dialog-actions"><button class="button" type="button" @click="dismissPaste">Cancel</button><button v-if="awaitingClipboard && clipboardReadable" class="button primary" type="button" :disabled="readingClipboard" @click="readClipboard">Retry clipboard</button><button v-if="!awaitingClipboard" class="button primary" :disabled="!enabled">{{ imageFile ? 'Upload' : 'Save file' }}</button></div>
    </form>
  </dialog>
  <dialog ref="conflictDialog" aria-label="File already exists" @cancel.prevent="resolve()"><h2>File already exists</h2><p>Choose what to do with “{{ conflict?.file.name }}”. Retrying uploads the file again from the beginning.</p><div class="dialog-actions"><button class="button" @click="resolve('keep')">Keep existing</button><button class="button" :disabled="!enabled" @click="resolve('auto_rename')">Save a copy</button><button class="button danger" :disabled="!enabled" @click="resolve('replace')">Replace</button></div></dialog>
  <button v-if="conflicts.length && !queue?.open" class="conflict-notice button" @click="show(queue)">{{ conflicts.length }} upload conflicts · Open queue</button>
</template>
