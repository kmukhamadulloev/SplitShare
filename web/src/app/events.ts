let source: EventSource | undefined
let retry: ReturnType<typeof setTimeout> | undefined
let delay = 500
const listeners = new Set<(event: MessageEvent) => void>()
const connections = new Set<(connected: boolean) => void>()
let online = false
function connection(value: boolean) { online = value; connections.forEach(listener => listener(value)) }
function connect() {
  if (!listeners.size || source || !navigator.onLine) return
  const current = new EventSource('/api/v1/events')
  source = current
  current.onopen = () => { if (source === current) { delay = 500; connection(true) } }
  current.onerror = () => {
    if (source !== current) return
    current.close(); source = undefined; connection(false)
    retry = setTimeout(() => { retry = undefined; connect() }, delay)
    delay = Math.min(delay * 2,10000)
  }
  for (const name of ['filesystem.changed','filesystem.resync','transfer.created','transfer.updated','transfer.resync','session.permissions_changed']) {
    current.addEventListener(name,event => { if (source === current) listeners.forEach(listener => listener(event as MessageEvent)) })
  }
}
function offline() {
  source?.close(); source = undefined; clearTimeout(retry); retry = undefined; connection(false)
}
function reconnect() {
  clearTimeout(retry); retry = undefined; delay = 500; connect()
}
export function observeEvents(event: (event: MessageEvent) => void, changed: (connected: boolean) => void = () => {}) {
  if (!listeners.size) {
    window.addEventListener('offline',offline)
    window.addEventListener('online',reconnect)
  }
  listeners.add(event); connections.add(changed); changed(online); connect()
  return () => {
    listeners.delete(event); connections.delete(changed)
    if (!listeners.size) { window.removeEventListener('offline',offline); window.removeEventListener('online',reconnect); source?.close(); source = undefined; clearTimeout(retry); retry = undefined; delay = 500; online = false }
  }
}
