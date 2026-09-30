export interface FileEntry { path: string; name: string; kind: 'file' | 'directory'; size: number | null; modified_unix_seconds: number | null }
export interface Permissions { browse: boolean; download: boolean; upload: boolean; create_directory: boolean; rename: boolean; delete: boolean }
export interface Status { sharing: boolean; local_client: boolean; root_label: string; share_mode: 'token_link' | 'open_lan'; upload_concurrency: number; permissions: Permissions }
export class ApiError extends Error {
  status: number
  code: string
  constructor(status: number, code: string, message: string) { super(message); this.status = status; this.code = code }
}
export async function request<T>(url: string, method = 'GET', body?: unknown, signal?: AbortSignal): Promise<T> {
  const response = await fetch(`/api/v1${url}`, { method, signal, headers: { ...(method !== 'GET' ? { 'X-SplitShare-Request': '1' } : {}), ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) })
  if (!response.ok) {
    const payload = await response.json().catch(() => null)
    throw new ApiError(response.status, payload?.error?.code ?? 'REQUEST_FAILED', payload?.error?.message ?? `Request failed (${response.status}).`)
  }
  return response.status === 204 ? undefined as T : await response.json() as T
}
export const downloadUrl = (path: string) => `/api/v1/files/download?${new URLSearchParams({ path })}`
