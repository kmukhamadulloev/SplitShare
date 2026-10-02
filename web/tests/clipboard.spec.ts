import {test,expect} from '@playwright/test'
import {networkInterfaces} from 'node:os'
import AxeBuilder from '@axe-core/playwright'
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aSuoAAAAASUVORK5CYII='

test('Paste opens one neutral editable modal without accessing the clipboard API', async ({page}) => {
  await page.addInitScript(() => Object.defineProperty(navigator,'clipboard',{get:() => {throw new Error('Clipboard API must not be accessed')}}))
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  const dialog = page.getByRole('dialog',{name:'Paste from clipboard',exact:true})
  await expect(dialog.getByRole('textbox',{name:'Paste area',exact:true})).toBeEditable()
  await expect(dialog.getByRole('alert')).toHaveCount(0)
  await expect(dialog.getByRole('button')).toHaveText(['Cancel'])
  await expect(dialog.getByLabel('Filename',{exact:true})).toHaveCount(0)
  await page.evaluate(() => Promise.all(document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).map(a => a.finished.catch(() => {}))))
  expect((await new AxeBuilder({page}).include('.paste-dialog').analyze()).violations).toEqual([])
  await page.screenshot({path:`test-results/simple-paste-${test.info().project.name}.png`})
  await dialog.getByRole('button',{name:'Cancel',exact:true}).click()
  await expect(page.getByRole('button',{name:'Paste',exact:true})).toBeFocused()
})

test('pasting image items transforms the same modal and preserves confirmed bytes', async ({page,request},info) => {
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await page.getByRole('textbox',{name:'Paste area',exact:true}).evaluate((element,png) => {
    const dialog = element.closest('dialog')!; dialog.dataset.originalModal = 'true'
    const file = new File([Uint8Array.from(atob(png),c=>c.charCodeAt(0))],'clipboard.png',{type:'image/png'})
    const event = new ClipboardEvent('paste',{bubbles:true,cancelable:true})
    Object.defineProperty(event,'clipboardData',{value:{files:[],items:[{kind:'file',getAsFile:()=>file}],getData:()=> 'image description'}})
    element.dispatchEvent(event)
  },png)
  const dialog = page.getByRole('dialog',{name:'Upload image',exact:true})
  await expect(dialog).toHaveAttribute('data-original-modal','true')
  await expect(dialog.getByRole('img',{name:'Image preview'})).toBeVisible()
  await expect(dialog.getByRole('textbox',{name:'Paste area',exact:true})).toHaveCount(0)
  const name = `pasted-${info.project.name}.png`
  await dialog.getByLabel('Filename',{exact:true}).fill(name)
  expect((await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(404)
  await dialog.getByRole('button',{name:'Upload',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(200)
  expect(await (await request.get(`/api/v1/files/download?path=/${name}`)).body()).toEqual(Buffer.from(png,'base64'))
})

test('text becomes an editable file draft and a cancelled draft is cleared', async ({page,request},info) => {
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await page.getByRole('textbox',{name:'Paste area',exact:true}).evaluate(element => {
    const data = new DataTransfer(); data.setData('text/plain','{"message":"original"}')
    const event = new ClipboardEvent('paste',{bubbles:true,cancelable:true})
    Object.defineProperty(event,'clipboardData',{value:data}); element.dispatchEvent(event)
  })
  const dialog = page.getByRole('dialog',{name:'Save clipboard text',exact:true})
  await expect(dialog.getByLabel('Filename',{exact:true})).toHaveValue('clipboard.json')
  await expect(dialog.getByLabel('Text',{exact:true})).toHaveValue('{"message":"original"}')
  const name = `text-${info.project.name}.md`
  await dialog.getByLabel('Filename',{exact:true}).fill(name)
  await dialog.getByLabel('Text',{exact:true}).fill('# Edited content')
  expect((await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(404)
  await dialog.getByRole('button',{name:'Save file',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).text()).toBe('# Edited content')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await page.getByRole('textbox',{name:'Paste area',exact:true}).fill('unsaved draft')
  await expect(dialog.getByLabel('Text',{exact:true})).toHaveValue('unsaved draft')
  await dialog.getByRole('button',{name:'Cancel',exact:true}).click()
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await expect(page.getByRole('textbox',{name:'Paste area',exact:true})).toHaveValue('')
  await expect(page.getByLabel('Filename',{exact:true})).toHaveCount(0)
})

test('empty and HTML-only paste do not fabricate files or fetch pasted URLs', async ({page}) => {
  await page.goto('/')
  const external: string[] = []
  page.on('request',r => {if (r.url().includes('invalid.example')) external.push(r.url())})
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  const target = page.getByRole('textbox',{name:'Paste area',exact:true})
  await target.evaluate(element => {
    const data = new DataTransfer(); data.setData('text/html','<img src="https://invalid.example/image.png"><script>window.injected=true</script>')
    const event = new ClipboardEvent('paste',{bubbles:true,cancelable:true})
    Object.defineProperty(event,'clipboardData',{value:data}); element.dispatchEvent(event)
  })
  await expect(target).toBeEditable()
  await expect(page.getByRole('alert')).toContainText('No image or text was provided')
  await expect(page.getByRole('button',{name:'Save file',exact:true})).toHaveCount(0)
  expect(external).toEqual([])
  expect(await page.evaluate(() => 'injected' in window)).toBe(false)
})

test('HTTP LAN uses the same neutral paste form without clipboard permissions', async ({browser,browserName}) => {
  const address = Object.values(networkInterfaces()).flat().find(item => item && item.family === 'IPv4' && !item.internal)?.address
  test.skip(!address,'Requires a LAN interface')
  const directBrowser = browserName === 'firefox' ? await browser.browserType().launch({firefoxUserPrefs:{'network.proxy.type':0}}) : undefined
  const context = await (directBrowser ?? browser).newContext(directBrowser ? {} : {proxy:{server:'http://127.0.0.1:9',bypass:address!}})
  try {
    const page = await context.newPage()
    await page.goto(`http://${address}:43123/`)
    expect(await page.evaluate(() => window.isSecureContext)).toBe(false)
    await page.getByRole('button',{name:'Paste',exact:true}).click()
    const dialog = page.getByRole('dialog',{name:'Paste from clipboard',exact:true})
    await expect(dialog.getByRole('textbox',{name:'Paste area',exact:true})).toBeEditable()
    await expect(dialog.getByRole('alert')).toHaveCount(0)
    await expect(dialog.getByRole('button')).toHaveText(['Cancel'])
  } finally { await context.close(); await directBrowser?.close() }
})

test('real Ctrl+V still imports an image without application clipboard reads', async ({page,context,browserName,request},info) => {
  test.skip(browserName !== 'chromium','Automated clipboard write permission is Chromium-only')
  await context.grantPermissions(['clipboard-read','clipboard-write'])
  await page.goto('/')
  await page.evaluate(async () => {
    const canvas = document.createElement('canvas'); canvas.width = 2; canvas.height = 2
    const drawing = canvas.getContext('2d')!; drawing.fillStyle = '#7c3aed'; drawing.fillRect(0,0,2,2)
    const blob = await new Promise<Blob>(resolve => canvas.toBlob(blob => resolve(blob!),'image/png'))
    await navigator.clipboard.write([new ClipboardItem({'image/png':blob})])
    Object.defineProperty(navigator.clipboard,'read',{value:() => {throw new Error('No programmatic reads')}})
  })
  await page.getByRole('button',{name:'Refresh files',exact:true}).focus()
  await page.keyboard.press('Control+v')
  const dialog = page.getByRole('dialog',{name:'Upload image',exact:true})
  await expect(dialog.getByRole('img',{name:'Image preview'})).toBeVisible()
  const name = `keyboard-${info.project.name}.png`
  await dialog.getByLabel('Filename',{exact:true}).fill(name)
  await dialog.getByRole('button',{name:'Upload',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(200)
})
