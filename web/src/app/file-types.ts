// Metadata only: never read file contents to select an icon.
export type FileCategory = 'folder' | 'image' | 'video' | 'audio' | 'pdf' | 'text' | 'code' | 'archive' | 'binary' | 'document' | 'spreadsheet' | 'presentation' | 'database' | 'font' | 'disk' | 'torrent' | 'certificate' | 'config' | 'unknown'
const extensions: Partial<Record<FileCategory, string[]>> = {
  image: ['png','jpg','jpeg','gif','webp','avif','svg','bmp','ico','heic','tiff'],
  video: ['mp4','webm','mkv','mov','avi','m4v'], audio: ['mp3','wav','flac','ogg','m4a','aac','opus'],
  pdf: ['pdf'], text: ['txt','md','rst','log'], code: ['rs','ts','tsx','js','jsx','vue','py','c','cpp','h','go','java','html','css','sh','swift','kt','rb','php'],
  archive: ['zip','tar','gz','bz2','xz','7z','rar','zst'], binary: ['exe','dll','so','bin','msi','app','deb','rpm'],
  document: ['doc','docx','odt','rtf','pages'], spreadsheet: ['xls','xlsx','ods','csv','numbers'], presentation: ['ppt','pptx','odp','keynote'],
  database: ['db','sqlite','sqlite3','sql'], font: ['ttf','otf','woff','woff2'], disk: ['iso','img','dmg','vhd','vhdx'], torrent: ['torrent'],
  certificate: ['pem','crt','cer','p12','pfx','key','pub'], config: ['json','yaml','yml','toml','ini','conf','env','xml'],
}
export function fileCategory(name: string, kind: string = 'file', mime = ''): FileCategory {
  if (kind === 'directory') return 'folder'
  const lower = name.toLowerCase()
  if (['dockerfile','makefile','cargo.lock','package-lock.json','.gitignore','.env'].includes(lower)) return 'config'
  const extension = lower.includes('.') ? lower.split('.').pop()! : ''
  for (const [category, values] of Object.entries(extensions)) if (values.includes(extension)) return category as FileCategory
  if (mime === 'application/pdf') return 'pdf'
  if (mime.startsWith('font/') || mime.includes('font-')) return 'font'
  if (mime.includes('spreadsheet') || mime.includes('ms-excel')) return 'spreadsheet'
  if (mime.includes('presentation') || mime.includes('ms-powerpoint')) return 'presentation'
  if (mime.includes('wordprocessing') || mime === 'application/msword') return 'document'
  if (['application/zip','application/gzip','application/x-tar','application/x-7z-compressed','application/x-rar-compressed'].includes(mime)) return 'archive'
  if (['application/json','application/xml','application/toml'].includes(mime)) return 'config'
  if (mime === 'application/x-bittorrent') return 'torrent'
  if (mime.includes('certificate') || mime.includes('x509')) return 'certificate'
  if (mime === 'application/octet-stream') return 'binary'
  if (mime.startsWith('image/')) return 'image'
  if (mime.startsWith('video/')) return 'video'
  if (mime.startsWith('audio/')) return 'audio'
  if (mime.startsWith('text/')) return 'text'
  return 'unknown'
}
export const typeLabels: Record<FileCategory,string> = { folder:'Folder', image:'Image', video:'Video', audio:'Audio', pdf:'PDF', text:'Text', code:'Source code', archive:'Archive', binary:'Application', document:'Document', spreadsheet:'Spreadsheet', presentation:'Presentation', database:'Database', font:'Font', disk:'Disk image', torrent:'Torrent', certificate:'Certificate / key', config:'Configuration', unknown:'File' }
export function formatBytes(bytes: number | null) {
  if (bytes === null) return '—'
  if (bytes < 1024) return `${bytes} B`
  const units = ['KiB','MiB','GiB','TiB','PiB']
  let value = bytes / 1024, unit = 0
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++ }
  return `${value.toFixed(1)} ${units[unit]}`
}
