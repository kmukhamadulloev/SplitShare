import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawn } from 'node:child_process'
const temporary = await mkdtemp(join(tmpdir(), 'splitshare-e2e-'))
const root = join(temporary, 'share')
await mkdir(root)
await writeFile(join(root, 'hello.txt'), 'Real shared file\n')
await mkdir(join(root, 'Documents'))
await mkdir(join(root, 'Many files'))
for (let base = 0; base < 10000; base += 100) {
  await Promise.all(Array.from({length:100},(_,index) => writeFile(join(root,'Many files',`file-${String(base+index).padStart(6,'0')}.txt`),'')))
}
await mkdir(join(root, 'Ordering'))
for (const name of ['z-folder', 'm-folder']) await mkdir(join(root, 'Ordering', name))
for (let index = 0; index < 100; index++) await writeFile(join(root, 'Ordering', `a-file-${String(index).padStart(3,'0')}.txt`), '')
const child = spawn(resolve('../target/debug/splitshare'), ['--no-tray', '--root', root, '--bind', '0.0.0.0:43123', '--parallel-uploads', '2'], { stdio: 'inherit', env: { ...process.env, XDG_DATA_HOME: join(temporary, 'config'), LOCALAPPDATA: join(temporary, 'config') } })
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal))
child.on('exit', async code => { await rm(temporary, { recursive: true, force: true }); process.exit(code ?? 0) })
