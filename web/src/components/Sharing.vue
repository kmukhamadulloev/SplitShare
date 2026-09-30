<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { QrCode, Settings, Copy, RefreshCw, LogOut } from '@lucide/vue'
import QRCode from 'qrcode'
import { request, type Permissions } from '../app/api'
import { useFiles } from '../app/files'
interface SettingsModel { share_mode: 'token_link' | 'open_lan'; permissions: Permissions; parallel_uploads_enabled: boolean; max_parallel_uploads: number }
interface Candidate { interface: string; address: string; kind: string; url: string }
const files = useFiles()
const settingsDialog = ref<HTMLDialogElement>()
const qrDialog = ref<HTMLDialogElement>()
const canvas = ref<HTMLCanvasElement>()
const model = ref<SettingsModel>()
const candidates = ref<Candidate[]>([])
const selected = ref('')
const tab = ref('access')
const error = ref('')
const notice = ref('')
const busy = ref(false)
const urlInput = ref<HTMLInputElement>()
const selectedCandidate = computed(() => candidates.value.find(candidate => candidate.url === selected.value))
const permissions: { key: keyof Permissions; label: string }[] = [{key:'browse',label:'Browse'},{key:'download',label:'Download'},{key:'upload',label:'Upload'},{key:'create_directory',label:'Create folders'},{key:'rename',label:'Rename'},{key:'delete',label:'Delete'}]
let focusBefore: HTMLElement | null = null
function show(dialog?: HTMLDialogElement) { focusBefore = document.activeElement as HTMLElement; dialog?.showModal() }
function close(dialog?: HTMLDialogElement) { if (!busy.value) { dialog?.close(); error.value = ''; notice.value = ''; focusBefore?.focus() } }
async function loadNetwork() {
  const previousAddress = selectedCandidate.value?.address
  candidates.value = (await request<{candidates: Candidate[]}>('/host/network')).candidates
  selected.value = candidates.value.find(candidate => candidate.address === previousAddress)?.url ?? candidates.value.find(candidate => candidate.kind !== 'loopback')?.url ?? candidates.value[0]?.url ?? ''
}
async function settings() {
  error.value = ''; notice.value = ''; show(settingsDialog.value); busy.value = true
  try { model.value = await request<SettingsModel>('/host/settings'); await loadNetwork() } catch (cause) { error.value = (cause as Error).message } finally { busy.value = false }
}
async function share() {
  error.value = ''; notice.value = ''; show(qrDialog.value)
  try { await loadNetwork(); await nextTick(); await draw() } catch (cause) { error.value = (cause as Error).message }
}
async function draw() {
  if (canvas.value && selected.value) {
    try { await QRCode.toCanvas(canvas.value, selected.value, { width: 260, margin: 4, errorCorrectionLevel: 'M' }) } catch { error.value = 'Could not generate this QR code.' }
  }
}
watch(selected, () => { void draw() })
async function save() {
  busy.value = true; error.value = ''
  try { await request('/host/settings','PUT',model.value); await files.refreshStatus(); busy.value = false; close(settingsDialog.value) } catch (cause) { error.value = (cause as Error).message } finally { busy.value = false }
}
async function rotate() {
  busy.value = true; error.value = ''
  try { await request('/host/share-token/rotate','POST'); await loadNetwork(); notice.value = 'New link created. Previous links and remote sessions are revoked.' } catch (cause) { error.value = (cause as Error).message } finally { busy.value = false }
}
async function copy() {
  try { await navigator.clipboard.writeText(selected.value); notice.value = 'Link copied.' }
  catch { urlInput.value?.focus(); urlInput.value?.select(); notice.value = 'Select and copy the link above.' }
}
async function leave() { try { await request('/session/leave','POST'); location.reload() } catch (cause) { files.error = (cause as Error).message } }
</script>
<template>
  <template v-if="files.status?.local_client">
    <button class="icon-btn" aria-label="Share with QR" :disabled="!files.status.sharing" @click="share"><QrCode :size="19" /></button>
    <button class="icon-btn" aria-label="Host settings" @click="settings"><Settings :size="19" /></button>
  </template>
  <button v-else-if="files.status?.share_mode === 'token_link'" class="icon-btn" aria-label="Leave share" @click="leave"><LogOut :size="19" /></button>
  <dialog ref="qrDialog" class="qr-dialog" aria-label="Share with QR" @cancel.prevent="close(qrDialog)">
    <h2>Share this folder</h2><p>Connect on the same trusted local or private network.</p>
    <label for="share-address">Network address</label>
    <select id="share-address" v-model="selected"><option v-for="candidate in candidates" :key="candidate.url" :value="candidate.url">{{ candidate.interface }} · {{ candidate.address }} · {{ candidate.kind }}</option></select>
    <p v-if="selectedCandidate?.kind === 'loopback'" class="error">This address works only on this host. {{ candidates.some(candidate => candidate.kind !== 'loopback') ? 'Select a LAN or VPN address for another device.' : 'Enable LAN binding when starting SplitShare to connect another device.' }}</p>
    <canvas v-show="selected" ref="canvas" aria-label="Share QR code" class="qr-canvas" />
    <input ref="urlInput" :value="selected" readonly aria-label="Share link" />
    <p v-if="!candidates.length">No compatible IPv4 address is available.</p>
    <p v-if="error" class="error" role="alert">{{ error }}</p><p role="status">{{ notice }}</p>
    <div class="dialog-actions"><button class="button" :disabled="!selected" @click="copy"><Copy :size="16" />Copy link</button><button class="button primary" @click="close(qrDialog)">Done</button></div>
  </dialog>
  <dialog ref="settingsDialog" class="settings-dialog" aria-label="Host settings" @cancel.prevent="close(settingsDialog)">
    <form @submit.prevent="save">
      <h2>Settings</h2><p>SplitShare host configuration</p>
      <div class="settings-layout">
        <nav class="settings-tabs" aria-label="Settings sections"><button v-for="section in ['access','transfers','network','security']" :key="section" type="button" :aria-pressed="tab === section" @click="tab = section">{{ section === 'access' ? 'Access & permissions' : section }}</button></nav>
        <div v-if="model" class="settings-content">
          <section v-show="tab === 'access'"><h3>Remote permissions</h3><p>These limits apply to remote clients. The host retains full control.</p><label v-for="permission in permissions" :key="permission.key" class="setting-toggle"><span>{{ permission.label }}</span><input v-model="model.permissions[permission.key]" type="checkbox" :aria-label="permission.label" /></label></section>
          <section v-show="tab === 'transfers'"><h3>Parallel uploads</h3><p>The server enforces this limit. Wait for the queue to finish before changing it.</p><label class="setting-toggle">Allow parallel uploads<input v-model="model.parallel_uploads_enabled" type="checkbox" /></label><label for="parallel-limit">Maximum parallel uploads</label><input id="parallel-limit" v-model.number="model.max_parallel_uploads" type="number" min="1" max="32" required /></section>
          <section v-show="tab === 'network'"><h3>Network addresses</h3><p>Select a reachable address in Share with QR. VPN and virtual adapters remain available.</p><p v-for="candidate in candidates" :key="candidate.url">{{ candidate.interface }} · {{ candidate.address }} · {{ candidate.kind }}</p><p>Listener address and port are chosen at startup.</p></section>
          <section v-show="tab === 'security'"><h3>Security</h3><label for="access-mode">Access mode</label><select id="access-mode" v-model="model.share_mode"><option value="token_link">QR / link token</option><option value="open_lan">Open on LAN</option></select><p v-if="model.share_mode === 'open_lan'" class="error">Any reachable client can connect. Remote permissions still apply.</p><p>Rotating the link disconnects remote sessions. A new token is also generated on restart.</p><button type="button" class="button" :disabled="busy" @click="rotate"><RefreshCw :size="16" />Rotate share token</button></section>
        </div>
      </div>
      <p v-if="error" class="error" role="alert">{{ error }}</p><p role="status">{{ notice }}</p>
      <div class="dialog-actions"><button type="button" class="button" :disabled="busy" @click="close(settingsDialog)">Cancel</button><button class="button primary" :disabled="busy || !model">Save changes</button></div>
    </form>
  </dialog>
</template>
