import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawn } from 'node:child_process'
const temporary = await mkdtemp(join(tmpdir(), 'splitshare-e2e-'))
const root = join(temporary, 'share')
await mkdir(root)
await writeFile(join(root, 'hello.txt'), 'Real shared file\n')
await mkdir(join(root, 'Documents'))
const child = spawn(resolve('../target/debug/splitshare'), ['--root', root, '--bind', '0.0.0.0:43123', '--parallel-uploads', '2'], { stdio: 'inherit', env: { ...process.env, XDG_DATA_HOME: join(temporary, 'config'), LOCALAPPDATA: join(temporary, 'config') } })
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal))
child.on('exit', async code => { await rm(temporary, { recursive: true, force: true }); process.exit(code ?? 0) })
