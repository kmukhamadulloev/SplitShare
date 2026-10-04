<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { FolderOpen, ArrowLeft, ArrowRight, Check, LoaderCircle } from '@lucide/vue'
import { request } from '../app/api'
import { useFiles } from '../app/files'
import { showDialog, closeDialog } from '../app/dialogs'
interface Setup { setup_required:boolean; folder_selected:boolean; bind_ip:string; port:number; state:string; message:string|null; local_url:string; interfaces:{address:string;label:string}[] }
const emit = defineEmits<{ ready: [] }>()
const files = useFiles()
const dialog = ref<HTMLDialogElement>()
const setup = ref<Setup>()
const step = ref(0), port = ref(8080), bind = ref('127.0.0.1')
const mode = ref('token_link'), access = ref('download'), error = ref(''), message = ref(''), busy = ref(false), advanced = ref(false), nextAddress = ref('')
const required = computed(() => files.status ? files.status.local_client && !files.status.sharing : !!dialog.value?.open)
const validPort = computed(() => Number.isInteger(port.value) && port.value >= 1 && port.value <= 65535)
let timer:ReturnType<typeof setTimeout>|undefined
let completing = false
function announceReady() { if (completing) { completing = false; emit('ready') } }
let generation=0, controller:AbortController|undefined
function stop() { generation++; clearTimeout(timer); controller?.abort() }
async function readSetup() {
  controller?.abort(); controller = new AbortController()
  const timeout = setTimeout(() => controller?.abort(),2500)
  try { return await request<Setup>('/host/setup','GET',undefined,controller.signal) }
  finally { clearTimeout(timeout) }
}
async function load() {
  busy.value = true; error.value = ''
  const current = generation
  let waiting = false
  try {
    const result = await readSetup()
    if (current !== generation) return
    if (!setup.value) { bind.value = result.bind_ip; port.value = result.port }
    setup.value = result; message.value = result.message ?? ''
    if (['selecting','applying'].includes(result.state)) { waiting = true; poll(result.state === 'applying'); return }
  } catch { if (current === generation) error.value = 'Could not load setup. Check the connection and try again.' }
  finally { if (current === generation && !waiting) busy.value = false }
}
function poll(waitForCompletion = false) {
  const current = generation
  busy.value = true
  let failures = 0, attempts = 0
  const check = async () => {
    if (current !== generation) return
    try {
      const result = await readSetup()
      if (current !== generation) return
      setup.value = result; failures = 0
      if (!['selecting','applying'].includes(result.state)) {
        busy.value = false; message.value = result.message ?? ''
        if (result.state === 'failed') { error.value = message.value; completing = false }
        else if (waitForCompletion && !result.setup_required) {
          if (nextAddress.value && new URL(nextAddress.value).origin !== location.origin) { location.assign(nextAddress.value); return }
          await files.refresh(); closeDialog(dialog.value); announceReady()
        }
        return
      }
    } catch { failures++ }
    if (current !== generation) return
    if (waitForCompletion && failures >= 6 && nextAddress.value) { location.assign(nextAddress.value); return }
    if (++attempts >= 620 || (!waitForCompletion && failures >= 10)) {
      busy.value = false; error.value = 'Could not confirm setup. Reopen SplitShare from the tray or retry loading setup.'; return
    }
    timer = setTimeout(() => void check(),500)
  }
  timer = setTimeout(() => void check(),500)
}
async function choose() {
  error.value = ''; busy.value = true
  try { setup.value = await request<Setup>('/host/folder','POST'); message.value = 'Choose a folder in the native dialog on this computer.'; poll() }
  catch (cause) { error.value = (cause as Error).message; busy.value = false }
}
async function finish() {
  if (!setup.value?.folder_selected || !validPort.value || busy.value) return
  error.value = ''; busy.value = true
  const ip = bind.value === '0.0.0.0' ? '127.0.0.1' : bind.value
  const target = `http://${ip}:${port.value}/#share`
  nextAddress.value = new URL(target).origin === location.origin ? '' : target
  try {
    completing = true
    setup.value = await request<Setup>('/host/setup/complete','POST',{
      network:{bind_ip:bind.value,port:port.value},
      settings:{share_mode:mode.value,parallel_uploads_enabled:false,max_parallel_uploads:3,
        permissions:{browse:true,download:true,upload:access.value === 'upload',create_directory:access.value === 'upload',rename:false,delete:false}}
    })
    message.value = 'Starting sharing. Applying your folder, network and access settings…'; poll(true)
  } catch (cause) { error.value = (cause as Error).message; busy.value = false; completing = false }
}
watch(required, async value => {
  if (value) { if (['#settings','#setup'].includes(location.hash)) history.replaceState(null,'',location.pathname + location.search); await nextTick(); if (!required.value) return; showDialog(dialog.value); void load() }
  else { stop(); closeDialog(dialog.value); announceReady() }
},{immediate:true})
onUnmounted(() => { stop(); closeDialog(dialog.value) })
</script>
<template>
  <dialog ref="dialog" class="setup-wizard" aria-label="Set up SplitShare" @cancel.prevent>
    <header><h2>Welcome to SplitShare</h2><p>Set up your shared folder before you start.</p></header>
    <ol class="setup-steps" aria-label="Setup progress"><li v-for="(label,index) in ['Folder','Connection','Access']" :key="label" :aria-current="step === index ? 'step' : undefined"><span>{{ index + 1 }}</span>{{ label }}</li></ol>
    <div class="setup-content">
      <section v-if="step === 0"><h3>Choose what to share</h3><p>Only files inside this folder will be available to connected devices. Nothing is shared until you finish setup.</p><button class="button primary" :disabled="busy || !setup" @click="choose"><FolderOpen :size="18" />{{ setup?.folder_selected ? 'Change shared folder' : 'Choose shared folder' }}</button><p v-if="setup?.folder_selected" class="setup-selected"><Check :size="18" />Folder selected</p><p>The folder picker opens on this computer. You can quit SplitShare from the tray at any time.</p></section>
      <section v-if="step === 1"><h3>Connect your devices</h3><label for="setup-network">Connection</label><select id="setup-network" v-model="bind" :disabled="busy"><option value="127.0.0.1">This computer only</option><option v-if="setup?.interfaces.some(item => item.address === '0.0.0.0')" value="0.0.0.0">Local network (LAN / VPN)</option><option v-if="!['127.0.0.1','0.0.0.0'].includes(bind)" :value="bind">{{ bind }}</option></select><p>Use Local network for phones, tablets and other computers on a trusted private network.</p><label for="setup-port">Port</label><input id="setup-port" v-model.number="port" type="number" min="1" max="65535" required :disabled="busy" /><p>Default: 8080. Changes apply when you start sharing.</p><button class="button" :aria-expanded="advanced" :disabled="busy" @click="advanced = !advanced">Advanced connection settings</button><template v-if="advanced"><label for="setup-interface">Network interface</label><select id="setup-interface" v-model="bind" :disabled="busy"><option v-for="item in setup?.interfaces" :key="item.address" :value="item.address">{{ item.label }}</option></select></template></section>
      <section v-if="step === 2"><h3>Choose access</h3><label for="setup-mode">Share access</label><select id="setup-mode" v-model="mode" :disabled="busy"><option value="token_link">Private link / QR (recommended)</option><option value="open_lan">Open LAN</option></select><p v-if="mode === 'open_lan'" class="error">Anyone who can reach this address can access the share with the permissions below.</p><p v-else>Devices join using your private link or QR code.</p><label for="setup-permissions">Permissions</label><select id="setup-permissions" v-model="access" :disabled="busy"><option value="download">Download only</option><option value="upload">Upload and download</option></select><p>Rename and delete are disabled for other devices. You can adjust permissions later in Settings.</p><div class="setup-summary"><strong>Ready to start</strong><p>Folder selected · {{ bind === '127.0.0.1' ? 'This computer only' : 'Local network' }} · Port {{ port }}</p></div></section>
      <p v-if="message" role="status">{{ message }}</p><p v-if="error" class="error" role="alert">{{ error }}</p><button v-if="error && !busy" class="button" @click="load">Reload setup status</button>
    </div>
    <footer class="dialog-actions"><span v-if="busy" class="setup-working"><LoaderCircle class="spinning" :size="18" />Working…</span><button v-if="step > 0" class="button" :disabled="busy" @click="step--"><ArrowLeft :size="16" />Back</button><button v-if="step < 2" class="button primary" :disabled="busy || !setup?.folder_selected || (step === 1 && !validPort)" @click="step++">Next<ArrowRight :size="16" /></button><button v-else class="button primary" :disabled="busy || !setup?.folder_selected || !validPort" @click="finish">Start sharing</button></footer>
  </dialog>
</template>
