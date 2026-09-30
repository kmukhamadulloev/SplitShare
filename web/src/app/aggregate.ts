export interface ProgressItem { total: number | null; transferred: number; state: string }
export function aggregate(items: ProgressItem[]) {
  const transferred = items.reduce((sum, item) => sum + item.transferred, 0)
  const total = items.every(item => item.total !== null) ? items.reduce((sum, item) => sum + item.total!, 0) : null
  const finished = items.length > 0 && items.every(item => item.state === 'completed')
  const percent = total === null ? null : total === 0 ? (finished ? 100 : 0) : Math.min(100, transferred / total * 100)
  return { transferred, total, percent, active: items.filter(item => ['uploading', 'publishing'].includes(item.state)).length, queued: items.filter(item => item.state === 'queued').length }
}
