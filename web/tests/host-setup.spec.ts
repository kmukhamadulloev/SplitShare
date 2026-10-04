import {test,expect} from '@playwright/test'
import {spawn} from 'node:child_process'
import {mkdtemp,rm,readFile} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join,resolve} from 'node:path'

test('first launch requires setup and cannot expose a folder or change network early', async ({page,request}) => {
  const config = await mkdtemp(join(tmpdir(),'splitshare-setup-'))
  const base = 'http://127.0.0.1:43130'
  const child = spawn(resolve('../target/debug/splitshare'),['--no-tray','--bind','127.0.0.1:43130'],{env:{...process.env,XDG_DATA_HOME:config,LOCALAPPDATA:config},stdio:'ignore'})
  const stop = async () => { if (child.exitCode !== null) return; const exited = new Promise(resolve => child.once('exit',resolve)); child.kill('SIGTERM'); await exited }
  try {
    await expect.poll(async () => { try {return (await request.get(`${base}/api/v1/status`)).status()} catch {return 0} }).toBe(200)
    await page.goto(base)
    const wizard = page.getByRole('dialog',{name:'Set up SplitShare',exact:true})
    await expect(wizard).toBeVisible()
    await expect(wizard.getByRole('button',{name:'Choose shared folder',exact:true})).toBeEnabled()
    await expect(wizard.getByRole('button',{name:'Next',exact:true})).toBeDisabled()
    await page.keyboard.press('Escape'); await expect(wizard).toBeVisible()
    await expect(wizard.getByRole('button',{name:/Skip|Cancel|Close/})).toHaveCount(0)
    await page.reload(); await expect(wizard).toBeVisible()
    expect((await request.get(`${base}/api/v1/files?path=/`)).status()).toBe(503)
    expect((await request.put(`${base}/api/v1/host/setup`,{headers:{'X-SplitShare-Request':'1'},data:{bind_ip:'0.0.0.0',port:43130}})).status()).toBe(409)
  } finally {await stop();await rm(config,{recursive:true,force:true})}
})

test('network changes move the browser automatically and survive restart', async ({page,request}) => {
  const config = await mkdtemp(join(tmpdir(),'splitshare-rebind-'))
  let child = spawn(resolve('../target/debug/splitshare'),['--no-tray','--root',config,'--bind','127.0.0.1:43130'],{env:{...process.env,XDG_DATA_HOME:config,LOCALAPPDATA:config},stdio:'ignore'})
  const stop = async () => { if (child.exitCode !== null) return; const exited = new Promise(resolve => child.once('exit',resolve)); child.kill('SIGTERM'); await exited }
  try {
    await expect.poll(async () => {try{return (await request.get('http://127.0.0.1:43130/api/v1/status')).status()}catch{return 0}}).toBe(200)
    await page.goto('http://127.0.0.1:43130')
    await expect(page.getByRole('dialog',{name:'Set up SplitShare',exact:true})).not.toBeVisible()
    await page.getByRole('button',{name:'Host settings',exact:true}).click()
    const settings = page.getByRole('dialog',{name:'Host settings',exact:true})
    await settings.getByRole('button',{name:'network',exact:true}).click()
    await settings.getByLabel('Port',{exact:true}).fill('0')
    await expect(settings.getByRole('button',{name:'Save changes',exact:true})).toBeDisabled()
    // The main E2E server occupies 43123: failure keeps the old address and entered value.
    await settings.getByLabel('Port',{exact:true}).fill('43123')
    await settings.getByRole('button',{name:'Save changes',exact:true}).click()
    await expect(settings.getByLabel('Host setup status')).toContainText('unavailable')
    await expect(settings.getByLabel('Port',{exact:true})).toHaveValue('43123')
    await settings.getByLabel('Port',{exact:true}).fill('43131')
    await settings.getByLabel('Interface',{exact:true}).selectOption('0.0.0.0')
    await settings.getByRole('button',{name:'Save changes',exact:true}).click()
    await expect(page).toHaveURL(/http:\/\/127\.0\.0\.1:43131\//,{timeout:20000})
    await expect(settings).toBeVisible()
    expect(JSON.parse(await readFile(join(config,'SplitShare','host.json'),'utf8')).bind).toBe('0.0.0.0:43131')
    await stop()
    child = spawn(resolve('../target/debug/splitshare'),['--no-tray'],{env:{...process.env,XDG_DATA_HOME:config,LOCALAPPDATA:config},stdio:'ignore'})
    await expect.poll(async () => {try{return (await request.get('http://127.0.0.1:43131/api/v1/status')).status()}catch{return 0}}).toBe(200)
    await page.goto('http://127.0.0.1:43131')
    await expect(page.getByRole('dialog',{name:'Set up SplitShare',exact:true})).not.toBeVisible()
  } finally {await stop();await rm(config,{recursive:true,force:true})}
})

test('QR errors can be retried and loopback sharing links to network settings', async ({page}) => {
  await page.goto('/')
  await page.route('**/api/v1/host/network',route => route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({error:{code:'UNAVAILABLE',message:'Network discovery unavailable.'}})}))
  await page.getByRole('button',{name:'Share with QR',exact:true}).click()
  const dialog = page.getByRole('dialog',{name:'Share with QR',exact:true})
  await expect(dialog.getByRole('alert')).toContainText('Network discovery unavailable')
  await expect(dialog.locator('canvas')).not.toBeVisible()
  await expect(dialog.getByLabel('Network address',{exact:true})).toHaveCount(0)
  await expect(dialog.getByLabel('Share link',{exact:true})).toHaveCount(0)
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
