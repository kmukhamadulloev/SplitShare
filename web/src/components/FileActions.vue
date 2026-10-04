<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue'
import { Download, FolderOpen, Pencil, Trash2, X } from '@lucide/vue'
import { downloadUrl, type FileEntry, type Permissions } from '../app/api'
import { showDialog, closeDialog } from '../app/dialogs'
const props = defineProps<{ permissions?: Permissions }>()
const emit = defineEmits<{ action: [kind: 'open' | 'rename' | 'delete', entry: FileEntry] }>()
const dialog = ref<HTMLDialogElement>()
const item = ref<FileEntry>()
const mobile = ref(false)
const position = ref({left:'8px',top:'8px'})
function close() { closeDialog(dialog.value) }
function resize() { mobile.value = matchMedia('(max-width: 767px)').matches; close() }
async function open(entry: FileEntry, event: MouseEvent | KeyboardEvent) {
  event.preventDefault(); item.value = entry
  mobile.value = matchMedia('(max-width: 767px)').matches
  const opener = event.currentTarget as HTMLElement
  opener?.focus({preventScroll:true})
  showDialog(dialog.value)
  await nextTick()
  if (dialog.value && !mobile.value) {
    const rect = dialog.value.getBoundingClientRect(), anchor = opener.getBoundingClientRect()
    const x = event instanceof MouseEvent && (event.type === 'contextmenu' || event.detail !== 0) ? event.clientX : anchor.left
    const y = event instanceof MouseEvent && (event.type === 'contextmenu' || event.detail !== 0) ? event.clientY : anchor.bottom
    position.value = {left:`${Math.max(8,Math.min(x,innerWidth-rect.width-8))}px`,top:`${Math.max(8,Math.min(y,innerHeight-rect.height-8))}px`}
  }
  dialog.value?.querySelector<HTMLElement>('[role="menuitem"]:not(:disabled)')?.focus()
}
function action(kind: 'open' | 'rename' | 'delete') { if (item.value) { close(); emit('action',kind,item.value) } }
function keyboard(event: KeyboardEvent) {
  if (!['ArrowDown','ArrowUp','Home','End'].includes(event.key)) return
  event.preventDefault()
  const controls = [...dialog.value?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? []]
  const index = controls.indexOf(document.activeElement as HTMLElement)
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? controls.length-1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + controls.length) % controls.length
  controls[next]?.focus()
}
onMounted(() => window.addEventListener('resize',resize))
onUnmounted(() => { window.removeEventListener('resize',resize); close() })
defineExpose({ open, close })
</script>
<template>
  <dialog ref="dialog" :class="mobile ? 'action-sheet' : 'context-menu'" :style="mobile ? {} : position" aria-label="Item actions" @cancel.prevent="close" @click="$event.target === dialog && close()" @keydown="keyboard">
    <div v-if="item" role="menu" aria-label="Item actions">
      <div class="sheet-handle" aria-hidden="true"></div><p class="menu-title">{{ item.name }}</p>
      <div class="menu-actions">
        <button v-if="item.kind === 'directory'" role="menuitem" :disabled="!props.permissions?.browse" @click="action('open')"><FolderOpen :size="19" /><span>Open folder</span></button>
        <button v-if="item.kind === 'file' && props.permissions?.download" role="menuitem" @click="action('open')"><FolderOpen :size="19" /><span>Open</span></button>
        <a v-if="item.kind === 'file' && props.permissions?.download" role="menuitem" :href="downloadUrl(item.path)" download @click="close"><Download :size="19" /><span>Download</span></a>
        <button role="menuitem" :disabled="!props.permissions?.rename" @click="action('rename')"><Pencil :size="19" /><span>Rename</span></button>
        <button role="menuitem" class="danger-text" :disabled="!props.permissions?.delete" @click="action('delete')"><Trash2 :size="19" /><span>Delete</span></button>
      </div>
      <button role="menuitem" class="menu-close" @click="close"><X :size="18" /><span>Close</span></button>
    </div>
  </dialog>
</template>
