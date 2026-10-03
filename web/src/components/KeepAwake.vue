<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
const props = defineProps<{ active: boolean }>()
const video = ref<HTMLVideoElement>()
const mode = ref<'off' | 'requesting' | 'native' | 'video' | 'interrupted' | 'error'>('off')
const useVideo = ref(false)
let lock: WakeLockSentinel | undefined
let generation = 0
function stop(next: typeof mode.value = 'off') {
  generation++
  const previous = lock; lock = undefined
  if (previous) void previous.release().catch(() => { console.warn('Could not release the screen wake lock.') })
  video.value?.pause()
  mode.value = next
}
async function enable() {
  if (!props.active || document.hidden) return
  if (['requesting','native','video'].includes(mode.value)) { stop(); return }
  const current = ++generation
  mode.value = 'requesting'
  try {
    if (!useVideo.value && window.isSecureContext && typeof navigator.wakeLock?.request === 'function') {
      const acquired = await navigator.wakeLock.request('screen')
      if (current !== generation) { await acquired.release(); return }
      lock = acquired
      mode.value = 'native'
      acquired.addEventListener('release', () => { if (lock === acquired) { lock = undefined; mode.value = 'interrupted' } }, {once:true})
    } else {
      if (!video.value) throw new Error('Video unavailable')
      // Call play synchronously from this tap; autoplay cannot be assumed on mobile.
      await video.value.play()
      if (current !== generation) return
      mode.value = 'video'
    }
  } catch {
    if (current !== generation) return
    useVideo.value = true
    stop('error')
  }
}
function interrupted() { if (mode.value === 'video') stop('interrupted') }
function visibility() { if (document.hidden && ['requesting','native','video'].includes(mode.value)) stop('interrupted') }
watch(() => props.active, active => { if (!active) stop() })
onMounted(() => document.addEventListener('visibilitychange', visibility))
onUnmounted(() => { document.removeEventListener('visibilitychange', visibility); stop() })
</script>
<template>
  <div v-if="active" class="keep-awake">
    <button class="button" type="button" :aria-pressed="['native','video'].includes(mode)" @click="enable">{{ ['requesting','native','video'].includes(mode) ? 'Stop keeping awake' : useVideo ? 'Try keeping awake' : 'Keep screen awake' }}</button>
    <span role="status">{{ mode === 'native' ? 'Screen wake lock active. Keep this page open.' : mode === 'video' ? 'Keep-awake fallback running. Your device may still lock.' : mode === 'requesting' ? 'Starting…' : mode === 'error' ? 'Could not keep the screen awake. Tap to try the video fallback, or keep the screen unlocked.' : mode === 'interrupted' ? 'Keep-awake stopped. Tap to enable it again.' : 'Keep this page visible and your screen unlocked during uploads.' }}</span>
  </div>
  <video ref="video" class="keep-awake-video" muted loop playsinline preload="none" aria-hidden="true" tabindex="-1" disablepictureinpicture @pause="interrupted" @error="interrupted">
    <source src="/media/keep-awake.mp4" type="video/mp4" />
    <source src="/media/keep-awake.webm" type="video/webm" />
  </video>
</template>
