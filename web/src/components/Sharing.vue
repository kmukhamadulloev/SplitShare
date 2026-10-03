<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { QrCode, Settings, Copy, RefreshCw, LogOut, FolderOpen } from '@lucide/vue'
import QRCode from 'qrcode'
import LogsDialog from './LogsDialog.vue'
import { request, type Permissions } from '../app/api'
import { useFiles } from '../app/files'
import { showDialog, closeDialog } from '../app/dialogs'
interface SettingsModel { share_mode: 'token_link' | 'open_lan'; permissions: Permissions; parallel_uploads_enabled: boolean; max_parallel_uploads: number }
interface HostSetup { bind_ip: string; port: number; folder_selected: boolean; interfaces: {address:string;label:string}[]; state: string; message: string | null; local_url: string }
const logsDialog = ref<InstanceType<typeof LogsDialog>>()
interface LogConfig { level: 'warn' | 'info' | 'debug' | null; available: boolean; capacity: number }
const logConfig = ref<LogConfig>()
const logLevel = ref('')
let initialSettings = ''
async function loadLogging() { logConfig.value = await request<LogConfig>('/host/logs/config'); logLevel.value = logConfig.value.level ?? '' }
const setup = ref<HostSetup>()
const bindIp = ref('127.0.0.1')
const port = ref(8080)
const setupBusy = ref(false)
const setupMessage = ref('')
const nextAddress = ref('')
const qrLoading = ref(false)
const qrReady = ref(false)
let poll: ReturnType<typeof setTimeout> | undefined
let polling = false
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
const show = showDialog
function close(dialog?: HTMLDialogElement) { if (!busy.value) { closeDialog(dialog); error.value = ''; notice.value = '' } }
async function loadNetwork() {
  const previousAddress = selectedCandidate.value?.address
  candidates.value = (await request<{candidates: Candidate[]}>('/host/network')).candidates
  selected.value = candidates.value.find(candidate => candidate.address === previousAddress)?.url ?? candidates.value.find(candidate => candidate.kind !== 'loopback')?.url ?? candidates.value[0]?.url ?? ''
}
async function settings() {
  error.value = ''; notice.value = ''; model.value = undefined; logConfig.value = undefined; logLevel.value = ''; show(settingsDialog.value); busy.value = true
  try { model.value = await request<SettingsModel>('/host/settings'); initialSettings = JSON.stringify(model.value); await Promise.all([loadNetwork(), loadSetup(), loadLogging()]); if (!files.status?.sharing) tab.value = 'general' } catch (cause) { error.value = (cause as Error).message } finally { busy.value = false }
}
async function share() {
  if (!files.status?.sharing) { await settings(); tab.value = 'general'; setupMessage.value = 'Choose a folder before generating a share link or QR code.'; return }
  error.value = ''; notice.value = ''; qrReady.value = false; qrLoading.value = true; selected.value = ''; candidates.value = []; show(qrDialog.value)
  try { await loadNetwork(); await nextTick(); await draw() } catch (cause) { error.value = (cause as Error).message } finally { qrLoading.value = false }
}
async function draw() {
  if (canvas.value && selected.value) {
    try { await QRCode.toCanvas(canvas.value, selected.value, { width: 260, margin: 4, errorCorrectionLevel: 'M' }); qrReady.value = true } catch { error.value = 'Could not generate this QR code.' }
  }
}
watch(selected, () => { qrReady.value = false; void draw() }, {flush:'post'})
async function save() {
  busy.value = true; error.value = ''
  try {
    if (JSON.stringify(model.value) !== initialSettings) {
      await request('/host/settings','PUT',model.value); initialSettings = JSON.stringify(model.value); await files.refreshStatus()
    }
    if (logLevel.value && logLevel.value !== logConfig.value?.level) {
      logConfig.value = await request<LogConfig>('/host/logs/config','PUT',{level:logLevel.value})
    }
    busy.value = false
    if (setup.value && (bindIp.value !== setup.value.bind_ip || port.value !== setup.value.port)) { await applyNetwork(); return }
    close(settingsDialog.value)
  } catch (cause) { error.value = (cause as Error).message } finally { busy.value = false }
}
async function rotate() {
  busy.value = true; error.value = ''
  try { await request('/host/share-token/rotate','POST'); await loadNetwork(); notice.value = 'New link created. Previous links and remote sessions are revoked.' } catch (cause) { error.value = (cause as Error).message } finally { busy.value = false }
}
async function copy() {
  try { await navigator.clipboard.writeText(selected.value); notice.value = 'Link copied.' }
  catch { urlInput.value?.focus(); urlInput.value?.select(); notice.value = 'Select and copy the link above.' }
}
function settingsRoute() {
  if (['#settings','#setup'].includes(location.hash) && files.status?.local_client && !settingsDialog.value?.open) {
    tab.value = location.hash === '#setup' ? 'general' : 'access'
    history.replaceState(null,'',location.pathname + location.search)
    void settings()
  }
}
watch(() => files.status?.local_client, settingsRoute)
onMounted(() => { window.addEventListener('hashchange', settingsRoute); settingsRoute() })
onUnmounted(() => { window.removeEventListener('hashchange', settingsRoute); polling = false; clearTimeout(poll) })
async function loadSetup() {
  setup.value = await request<HostSetup>('/host/setup')
  bindIp.value = setup.value.bind_ip; port.value = setup.value.port
  setupMessage.value = setup.value.message ?? ''
  setupBusy.value = ['selecting','applying'].includes(setup.value.state)
  if (setupBusy.value) startPolling()
}
function startPolling() {
  if (polling) return
  polling = true
  let failures = 0
  const check = async () => {
    if (!polling) return
    try {
      const current = await request<HostSetup>('/host/setup')
      if (!polling) return
      setup.value = current; failures = 0
      if (!['selecting','applying'].includes(current.state)) {
        polling = false; setupBusy.value = false; nextAddress.value = ''
        bindIp.value = current.bind_ip; port.value = current.port
        setupMessage.value = current.message ?? ''
        await files.refreshStatus(); await files.load('/'); await loadNetwork().catch(cause => { error.value = (cause as Error).message }); return
      }
    } catch { failures++ }
    if (failures >= 20) {
      polling = false; setupBusy.value = false
      setupMessage.value = nextAddress.value ? 'The connection moved or was interrupted. Open the requested address below to check the change. If it is unavailable, reopen SplitShare from the tray.' : 'Could not confirm the change. Reopen SplitShare from the tray or refresh this page.'
      return
    }
    poll = setTimeout(() => { void check() }, 500)
  }
  poll = setTimeout(() => { void check() }, 500)
}
async function chooseFolder() {
  error.value = ''; setupMessage.value = ''; nextAddress.value = ''
  try { setup.value = await request<HostSetup>('/host/folder','POST'); setupBusy.value = true; setupMessage.value = 'Choose a folder in the native dialog on this computer. This page will update after selection.'; startPolling() }
  catch (cause) { error.value = (cause as Error).message }
}
async function applyNetwork() {
  error.value = ''; setupMessage.value = ''
  try {
    const nextIp = bindIp.value === '0.0.0.0' ? '127.0.0.1' : bindIp.value
    const nextUrl = `http://${nextIp}:${port.value}/#settings`
    setup.value = await request<HostSetup>('/host/setup','PUT',{bind_ip:bindIp.value,port:port.value})
    setupBusy.value = true
    nextAddress.value = new URL(nextUrl).origin !== location.origin ? nextUrl : ''
    setupMessage.value = 'Applying network settings. Existing transfers and sessions will disconnect.'
    startPolling()
  } catch (cause) { error.value = (cause as Error).message }
}
async function networkSettings() { close(qrDialog.value); await settings(); tab.value = 'network' }
async function leave() { try { await request('/session/leave','POST'); location.reload() } catch (cause) { files.error = (cause as Error).message } }
</script>
<template>
  <template v-if="files.status?.local_client">
    <button class="icon-btn" aria-label="Share with QR" :disabled="setupBusy" @click="share"><QrCode :size="19" /></button>
    <button class="icon-btn" aria-label="Host settings" @click="settings"><Settings :size="19" /></button>
  </template>
  <button v-else-if="files.status?.share_mode === 'token_link'" class="icon-btn" aria-label="Leave share" @click="leave"><LogOut :size="19" /></button>
  <dialog ref="qrDialog" class="qr-dialog" aria-label="Share with QR" @cancel.prevent="close(qrDialog)">
    <h2>Share this folder</h2><p>Connect on the same trusted local or private network.</p>
    <template v-if="candidates.length">
    <label for="share-address">Network address</label>
    <select id="share-address" v-model="selected"><option v-for="candidate in candidates" :key="candidate.url" :value="candidate.url">{{ candidate.interface }} · {{ candidate.address }} · {{ candidate.kind }}</option></select>
    </template>
    <p v-if="selectedCandidate?.kind === 'loopback'" class="error">This address works only on this host. {{ candidates.some(candidate => candidate.kind !== 'loopback') ? 'Select a LAN or VPN address for another device.' : 'Enable All interfaces in Network settings to connect another device.' }}</p>
    <p v-if="qrLoading" role="status">Generating QR code…</p>
    <button v-if="selectedCandidate?.kind === 'loopback'" class="button" @click="networkSettings">Network settings</button>
    <canvas v-show="qrReady" ref="canvas" aria-label="Share QR code" class="qr-canvas" />
    <input v-if="selected" ref="urlInput" :value="selected" readonly aria-label="Share link" />
    <p v-if="!qrLoading && !candidates.length">No compatible IPv4 address is available.</p>
    <p v-if="error" class="error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
    <div class="dialog-actions"><button class="button" :disabled="qrLoading" @click="share">Refresh QR</button><button class="button" :disabled="!selected || !qrReady" @click="copy"><Copy :size="16" />Copy link</button><button class="button primary" @click="close(qrDialog)">Done</button></div>
  </dialog>
  <dialog ref="settingsDialog" class="settings-dialog" aria-label="Host settings" @cancel.prevent="close(settingsDialog)">
    <form @submit.prevent="save">
      <h2>Settings</h2><p>SplitShare host configuration</p>
      <div class="settings-layout">
        <nav class="settings-tabs" aria-label="Settings sections"><button v-for="section in ['general','access','transfers','network','security','logging']" :key="section" type="button" :aria-pressed="tab === section" @click="tab = section">{{ section === 'access' ? 'Access & permissions' : section }}</button></nav>
        <div v-if="!model" class="settings-content" role="status">{{ busy ? 'Loading settings…' : 'Settings could not be loaded. Close and try again.' }}</div>
        <div v-else class="settings-content">
          <section v-show="tab === 'general'"><h3>Shared folder</h3><p>{{ setup?.folder_selected ? 'A folder is selected for sharing.' : 'No folder selected. Choose the folder whose contents you want to share.' }}</p><p>The native dialog opens on this computer. Its filesystem path stays private.</p><button type="button" class="button" :disabled="setupBusy || !setup" @click="chooseFolder"><FolderOpen :size="18" />Choose shared folder</button><p>Changing folders disconnects clients and cancels active transfers. Your choice is saved for the next launch.</p></section>
          <section v-show="tab === 'access'"><h3>Remote permissions</h3><p>These limits apply to remote clients. The host retains full control.</p><label v-for="permission in permissions" :key="permission.key" class="setting-toggle"><span>{{ permission.label }}</span><input v-model="model.permissions[permission.key]" type="checkbox" :aria-label="permission.label" /></label></section>
          <section v-show="tab === 'transfers'"><h3>Parallel uploads</h3><p>The server enforces this limit. Wait for the queue to finish before changing it.</p><label class="setting-toggle">Allow parallel uploads<input v-model="model.parallel_uploads_enabled" type="checkbox" /></label><label for="parallel-limit">Maximum parallel uploads</label><input id="parallel-limit" v-model.number="model.max_parallel_uploads" type="number" min="1" max="32" required /></section>
          <section v-show="tab === 'network'"><h3>Network</h3><p>Choose where SplitShare listens. All interfaces enables LAN, Ethernet, hotspot and VPN access; token protection still applies.</p><label for="bind-interface">Interface</label><select id="bind-interface" v-model="bindIp" :disabled="setupBusy || !setup"><option v-for="item in setup?.interfaces" :key="item.address" :value="item.address">{{ item.label }} · {{ item.address }}</option></select><label for="bind-port">Port</label><input id="bind-port" v-model.number="port" type="number" min="1" max="65535" required :disabled="setupBusy || !setup" /><p>Applying restarts the listener and disconnects clients and transfers. Changes are saved for the next launch.</p></section>
          <section v-show="tab === 'logging'"><h3>Logging</h3><label for="logging-level">Logging level</label><select id="logging-level" v-model="logLevel" :disabled="!logConfig?.available"><option value="" disabled>Startup configuration</option><option value="warn">Warnings and errors</option><option value="info">Standard</option><option value="debug">Detailed</option></select><p>Save changes applies the level to the console and log viewer for this run. Restarting restores the startup configuration.</p><p>History holds the latest 500 entries in memory. Only the host can view or clear it.</p><button type="button" class="button" @click="logsDialog?.open()">Open logs</button></section>
          <section v-show="tab === 'security'"><h3>Security</h3><label for="access-mode">Access mode</label><select id="access-mode" v-model="model.share_mode"><option value="token_link">QR / link token</option><option value="open_lan">Open on LAN</option></select><p v-if="model.share_mode === 'open_lan'" class="error">Any reachable client can connect. Remote permissions still apply.</p><p>Rotating the link disconnects remote sessions. A new token is also generated on restart.</p><button type="button" class="button" :disabled="busy" @click="rotate"><RefreshCw :size="16" />Rotate share token</button></section>
        </div>
      </div>
      <p v-if="setupMessage" aria-label="Host setup status" aria-live="polite">{{ setupMessage }}</p><p v-if="nextAddress && !setupBusy"><a :href="nextAddress" class="button">Open updated address</a></p>
      <p v-if="error" class="error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
      <div class="dialog-actions"><button type="button" class="button" :disabled="busy" @click="close(settingsDialog)">Cancel</button><button class="button primary" :disabled="busy || setupBusy || !model || !setup || !Number.isInteger(port) || port < 1 || port > 65535">Save changes</button></div>
    </form>
  </dialog>
  <LogsDialog v-if="files.status?.local_client" ref="logsDialog" />
</template>
