import {test,expect} from '@playwright/test'
import {networkInterfaces} from 'node:os'
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aSuoAAAAASUVORK5CYII='

test('image paste works inside clipboard permission fallback and preserves bytes', async ({page,request},info) => {
  await page.addInitScript(() => Object.defineProperty(navigator,'clipboard',{value:{read:async () => {throw new Error('Denied')}}}))
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await expect(page.getByRole('alert')).toContainText('could not read the clipboard')
  await expect(page.getByRole('button',{name:'Save file',exact:true})).toHaveCount(0)
  await page.getByRole('dialog',{name:'Paste from clipboard',exact:true}).evaluate((element,png) => {
    const file = new File([Uint8Array.from(atob(png),c=>c.charCodeAt(0))],'clipboard.png',{type:'image/png'})
    const event = new ClipboardEvent('paste',{bubbles:true,cancelable:true})
    // Exercise browsers exposing a file item without populating files.
    Object.defineProperty(event,'clipboardData',{value:{files:[],items:[{kind:'file',getAsFile:()=>file}],getData:()=> 'image description'}})
    element.dispatchEvent(event)
  },png)
  await expect(page.getByRole('dialog',{name:'Paste image',exact:true})).toBeVisible()
  await expect(page.getByRole('img',{name:'Pasted image preview'})).toBeVisible()
  const name = `fallback-${info.project.name}.png`
  await page.getByLabel('Filename',{exact:true}).fill(name)
  expect((await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(404)
  await page.getByRole('button',{name:'Save file',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(200)
  expect(await (await request.get(`/api/v1/files/download?path=/${name}`)).body()).toEqual(Buffer.from(png,'base64'))
})

for (const imageFirst of [true,false]) test(`clipboard image takes priority over separate text items (${imageFirst})`, async ({page}) => {
  await page.addInitScript(({png,imageFirst}) => {
    const image = {types:['image/png'],getType:async () => new Blob([Uint8Array.from(atob(png),c=>c.charCodeAt(0))],{type:'image/png'})}
    const text = {types:['text/plain'],getType:async () => new Blob(['image description'],{type:'text/plain'})}
    Object.defineProperty(navigator,'clipboard',{value:{read:async () => imageFirst ? [image,text] : [text,image]}})
  },{png,imageFirst})
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await expect(page.getByRole('dialog',{name:'Paste image',exact:true})).toBeVisible()
  await expect(page.getByRole('img',{name:'Pasted image preview'})).toBeVisible()
  await expect(page.getByLabel('Filename',{exact:true})).toHaveValue('clipboard.png')
  await expect(page.getByLabel('Text',{exact:true})).toHaveCount(0)
})

test('Paste button reads an image through the real browser Clipboard API', async ({page,context,browserName,request},info) => {
  test.skip(browserName !== 'chromium','Automated clipboard permission grant is Chromium-only')
  await context.grantPermissions(['clipboard-read','clipboard-write'])
  await page.goto('/')
  const expected = await page.evaluate(async () => {
    const canvas = document.createElement('canvas'); canvas.width = 2; canvas.height = 2
    const drawing = canvas.getContext('2d')!; drawing.fillStyle = '#7c3aed'; drawing.fillRect(0,0,2,2)
    const blob = await new Promise<Blob>(resolve => canvas.toBlob(blob => resolve(blob!),'image/png'))
    await navigator.clipboard.write([new ClipboardItem({'image/png':blob})])
    const item = (await navigator.clipboard.read())[0]!
    return Array.from(new Uint8Array(await (await item.getType('image/png')).arrayBuffer()))
  })
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await expect(page.getByRole('dialog',{name:'Paste image',exact:true})).toBeVisible()
  await expect(page.getByRole('img',{name:'Pasted image preview'})).toBeVisible()
  const name = `real-button-${info.project.name}.png`
  await page.getByLabel('Filename',{exact:true}).fill(name)
  await page.getByRole('button',{name:'Save file',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(200)
  expect(await (await request.get(`/api/v1/files/download?path=/${name}`)).body()).toEqual(Buffer.from(expected))
})


test('Paste button on HTTP LAN never misclassifies blocked access as text', async ({browser,browserName}) => {
  const address = Object.values(networkInterfaces()).flat().find(item => item && item.family === 'IPv4' && !item.internal)?.address
  test.skip(!address,'Requires a LAN interface')
  // Explicit bypass keeps desktop/system proxies out of this local-network test.
  const directBrowser = browserName === 'firefox' ? await browser.browserType().launch({firefoxUserPrefs:{'network.proxy.type':0}}) : undefined
  const context = await (directBrowser ?? browser).newContext(directBrowser ? {} : {proxy:{server:'http://127.0.0.1:9',bypass:address!}})
  try {
    const page = await context.newPage()
    await page.goto(`http://${address}:43123/`)
    expect(await page.evaluate(() => window.isSecureContext)).toBe(false)
    await page.getByRole('button',{name:'Paste',exact:true}).click()
    const dialog = page.getByRole('dialog',{name:'Paste from clipboard',exact:true})
    await expect(dialog).toBeVisible()
    await expect(dialog.getByRole('alert')).toContainText('This HTTP address')
    await expect(dialog.getByRole('textbox')).toHaveCount(0)
    await expect(dialog.getByRole('button',{name:'Save file',exact:true})).toHaveCount(0)
  } finally { await context.close(); await directBrowser?.close() }
})

test('Paste button can retry after clipboard permission is granted', async ({page}) => {
  await page.addInitScript(png => {
    let attempts = 0
    Object.defineProperty(navigator,'clipboard',{value:{read:async () => {
      if (!attempts++) throw new DOMException('Denied','NotAllowedError')
      return [{types:['image/png'],getType:async () => new Blob([Uint8Array.from(atob(png),c=>c.charCodeAt(0))],{type:'image/png'})}]
    }}})
  },png)
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await expect(page.getByRole('alert')).toContainText('Clipboard permission was denied')
  await page.getByRole('button',{name:'Retry clipboard',exact:true}).click()
  await expect(page.getByRole('dialog',{name:'Paste image',exact:true})).toBeVisible()
  await expect(page.getByRole('img',{name:'Pasted image preview'})).toBeVisible()
})
