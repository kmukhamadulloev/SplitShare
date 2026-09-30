export interface FileEntry { path: string; name: string; kind: 'file' | 'directory'; size: number | null; modified_unix_seconds: number | null }
export interface Permissions { browse: boolean; download: boolean; upload: boolean; create_directory: boolean; rename: boolean; delete: boolean }
export interface Status { sharing: boolean; local_client: boolean; root_label: string; share_mode: 'token_link' | 'open_lan'; upload_concurrency: number; permissions: { browse: boolean; download: boolean; upload: boolean; create_directory: boolean; rename: boolean; delete: boolean } }
export async function request<T>(url: string, method = 'GET', body?: unknown): Promise<T> {
  const response = await fetch(`/api/v1${url}`, { method, headers: { ...(method !== 'GET' ? { 'X-SplitShare-Request': '1' } : {}), ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) })
  if (!response.ok) {
    const payload = await response.json().catch(() => null)
    throw new Error(payload?.error?.message ?? `Request failed (${response.status}).`)
  }
  return response.status === 204 ? undefined as T : await response.json() as T
}
export const downloadUrl = (path: string) => `/api/v1/files/download?${new URLSearchParams({ path })}`
