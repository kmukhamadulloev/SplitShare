import {test,expect} from '@playwright/test'
import {spawn} from 'node:child_process'
import {mkdtemp,rm,readFile} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join,resolve} from 'node:path'

test('first launch offers folder setup and applies persistent LAN settings', async ({page,request}) => {
  const config = await mkdtemp(join(tmpdir(),'splitshare-setup-'))
  const base = 'http://127.0.0.1:43130'
  let child = spawn(resolve('../target/debug/splitshare'),['--no-tray','--bind','127.0.0.1:43130'],{env:{...process.env,XDG_DATA_HOME:config,LOCALAPPDATA:config},stdio:'ignore'})
  const stop = async () => { if (child.exitCode !== null) return; const exited = new Promise(resolve => child.once('exit',resolve)); child.kill('SIGTERM'); await exited }
  try {
    await expect.poll(async () => { try {return (await request.get(`${base}/api/v1/status`)).status()} catch {return 0} }).toBe(200)
    await page.goto(base)
    await expect(page.getByRole('heading',{name:'No folder is being shared'})).toBeVisible()
    await page.getByRole('link',{name:'Set up sharing'}).click()
    const settings = page.getByRole('dialog',{name:'Host settings',exact:true})
    await expect(settings.getByRole('button',{name:'Choose shared folder',exact:true})).toBeEnabled()
    await expect(settings.getByText('No folder selected.',{exact:false})).toBeVisible()
    await settings.getByRole('button',{name:'Cancel',exact:true}).click()
    await page.getByRole('button',{name:'Share with QR',exact:true}).click()
    await expect(settings.getByLabel('Host setup status')).toContainText('Choose a folder before generating')
    await settings.getByRole('button',{name:'network',exact:true}).click()
    await settings.getByLabel('Interface',{exact:true}).selectOption('0.0.0.0')
    await settings.getByRole('button',{name:'Apply network settings'}).click()
    await expect(settings.getByLabel('Host setup status')).toContainText('Host setup saved',{timeout:15000})
    expect((await (await request.get(`${base}/api/v1/host/setup`)).json()).bind_ip).toBe('0.0.0.0')
    await settings.getByLabel('Interface',{exact:true}).selectOption('127.0.0.1')
    await settings.getByRole('button',{name:'Save changes',exact:true}).click()
    await expect.poll(async () => {try {return (await (await request.get(`${base}/api/v1/host/setup`)).json()).bind_ip} catch {return ''}}).toBe('127.0.0.1')
    await expect(settings.getByRole('button',{name:'Apply network settings'})).toBeEnabled()
    await settings.getByLabel('Interface',{exact:true}).selectOption('0.0.0.0')
    await settings.getByRole('button',{name:'Apply network settings'}).click()
    await expect.poll(async () => {try {return (await (await request.get(`${base}/api/v1/host/setup`)).json()).bind_ip} catch {return ''}}).toBe('0.0.0.0')
    expect(JSON.parse(await readFile(join(config,'SplitShare','host.json'),'utf8')).bind).toBe('0.0.0.0:43130')
    await stop()
    child = spawn(resolve('../target/debug/splitshare'),['--no-tray'],{env:{...process.env,XDG_DATA_HOME:config,LOCALAPPDATA:config},stdio:'ignore'})
    await expect.poll(async () => {try {return (await (await request.get(`${base}/api/v1/host/setup`)).json()).bind_ip} catch {return ''}}).toBe('0.0.0.0')
    await page.goto(base)
    await expect(page.getByRole('link',{name:'Set up sharing'})).toBeVisible()
  } finally {await stop();await rm(config,{recursive:true,force:true})}
})

test('QR errors can be retried and loopback sharing links to network settings', async ({page}) => {
  await page.goto('/')
  await page.route('**/api/v1/host/network',route => route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({error:{code:'UNAVAILABLE',message:'Network discovery unavailable.'}})}))
  await page.getByRole('button',{name:'Share with QR',exact:true}).click()
  const dialog = page.getByRole('dialog',{name:'Share with QR',exact:true})
  await expect(dialog.getByRole('alert')).toContainText('Network discovery unavailable')
  await expect(dialog.locator('canvas')).not.toBeVisible()
  await page.unroute('**/api/v1/host/network')
  await dialog.getByRole('button',{name:'Refresh QR',exact:true}).click()
  await expect(dialog.locator('canvas')).toBeVisible()
  const loopback = await dialog.getByLabel('Network address',{exact:true}).locator('option').evaluateAll(options => options.map(option => (option as HTMLOptionElement).value).find(value => value.startsWith('http://127.0.0.1:'))!)
  await dialog.getByLabel('Network address',{exact:true}).selectOption(loopback)
  await dialog.getByRole('button',{name:'Network settings',exact:true}).click()
  const settings = page.getByRole('dialog',{name:'Host settings',exact:true})
  await expect(settings.getByLabel('Interface',{exact:true})).toBeVisible()
  await expect(settings.getByLabel('Port',{exact:true})).toBeVisible()
})
