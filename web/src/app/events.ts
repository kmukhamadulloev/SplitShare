let source: EventSource | undefined
const listeners = new Set<(event: MessageEvent) => void>()
const connections = new Set<(connected: boolean) => void>()
let online = false
export function observeEvents(event: (event: MessageEvent) => void, connection: (connected: boolean) => void = () => {}) {
  listeners.add(event); connections.add(connection); connection(online)
  if (!source) {
    source = new EventSource('/api/v1/events')
    source.onopen = () => { online = true; connections.forEach(listener => listener(true)) }
    source.onerror = () => { online = false; connections.forEach(listener => listener(false)) }
    for (const name of ['filesystem.changed', 'filesystem.resync', 'transfer.created', 'transfer.updated', 'transfer.resync']) source.addEventListener(name, incoming => listeners.forEach(listener => listener(incoming as MessageEvent)))
  }
  return () => {
    listeners.delete(event); connections.delete(connection)
    if (!listeners.size) { source?.close(); source = undefined; online = false }
  }
}
