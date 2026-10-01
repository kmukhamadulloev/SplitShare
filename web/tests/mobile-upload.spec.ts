import {test,expect} from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aSuoAAAAASUVORK5CYII='

test('mobile upload sheet uses media and file pickers, confirming images before upload', async ({page,request},info) => {
  await page.setViewportSize({width:390,height:844})
  await page.goto('/')
  const upload = page.getByRole('button',{name:'Upload files',exact:true})
  await upload.click()
  const sheet = page.getByRole('dialog',{name:'Upload',exact:true})
  await expect(sheet).toBeVisible()
  await expect(sheet.getByRole('button',{name:'Paste from clipboard'})).toBeVisible()
  await expect.poll(() => sheet.evaluate(element => Math.abs(element.getBoundingClientRect().bottom - innerHeight))).toBeLessThan(2)
  await page.screenshot({path:`test-results/upload-sheet-${info.project.name}.png`})
  const choose = page.waitForEvent('filechooser')
  await sheet.getByRole('button',{name:'Photos & videos',exact:true}).click()
  const media = await choose
  expect(await media.element().getAttribute('accept')).toBe('image/*,video/*')
  expect(media.isMultiple()).toBe(true)
  await media.setFiles([{name:`photo-${info.project.name}.png`,mimeType:'image/png',buffer:Buffer.from(png,'base64')}, {name:`cancel-photo-${info.project.name}.png`,mimeType:'image/png',buffer:Buffer.from(png,'base64')}])
  const preview = page.getByRole('dialog',{name:'Upload image',exact:true})
  await expect(preview.getByRole('img',{name:'Image preview'})).toBeVisible()
  await expect(sheet).not.toBeVisible()
  const name = `confirmed-photo-${info.project.name}.png`
  await preview.getByLabel('Filename',{exact:true}).fill(name)
  expect((await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(404)
  await preview.getByRole('button',{name:'Upload',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(200)
  expect(await (await request.get(`/api/v1/files/download?path=/${name}`)).body()).toEqual(Buffer.from(png,'base64'))
  await expect(preview.getByLabel('Filename',{exact:true})).toHaveValue(`cancel-photo-${info.project.name}.png`)
  await preview.getByRole('button',{name:'Cancel',exact:true}).click()
  expect((await request.get(`/api/v1/files/download?path=/cancel-photo-${info.project.name}.png`)).status()).toBe(404)
  await upload.click()
  const browse = page.waitForEvent('filechooser')
  await sheet.getByRole('button',{name:'Browse files',exact:true}).click()
  const files = await browse
  expect(await files.element().getAttribute('accept')).toBeNull()
  const document = `mobile-document-${info.project.name}.txt`
  await files.setFiles({name:document,mimeType:'text/plain',buffer:Buffer.from('from native picker')})
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${document}`)).text()).toBe('from native picker')
})

test('mobile clipboard fallback is editable, accessible and offers photo selection', async ({page,request},info) => {
  await page.setViewportSize({width:390,height:844})
  await page.addInitScript(() => Object.defineProperty(navigator,'clipboard',{value:undefined}))
  await page.goto('/')
  await page.getByRole('button',{name:'Upload files',exact:true}).click()
  await page.getByRole('button',{name:'Paste from clipboard',exact:true}).click()
  const dialog = page.getByRole('dialog',{name:'Paste from clipboard',exact:true})
  const target = dialog.getByRole('textbox',{name:'Paste area',exact:true})
  await expect(target).toBeEditable()
  await expect(dialog.getByRole('button',{name:'Retry clipboard',exact:true})).toHaveCount(0)
  await expect(dialog.getByText('Touch and hold here, then choose Paste.',{exact:true})).toBeVisible()
  await expect(dialog.getByRole('button',{name:'Save file',exact:true})).toHaveCount(0)
  await page.evaluate(() => Promise.all(document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).map(a => a.finished.catch(() => {}))))
  expect((await new AxeBuilder({page}).include('.paste-dialog').analyze()).violations).toEqual([])
  await page.screenshot({path:`test-results/upload-paste-${info.project.name}.png`})
  // A browser-supplied image event at the actual editable target becomes a preview.
  await target.evaluate((element,png) => {
    const data = new DataTransfer()
    data.items.add(new File([Uint8Array.from(atob(png),c=>c.charCodeAt(0))],'long-press.png',{type:'image/png'}))
    data.setData('text/html','<img src="https://invalid.example/must-not-fetch">')
    const event = new ClipboardEvent('paste',{bubbles:true,cancelable:true})
    Object.defineProperty(event,'clipboardData',{value:data}); element.dispatchEvent(event)
  },png)
  const preview = page.getByRole('dialog',{name:'Upload image',exact:true})
  await expect(preview.getByRole('img',{name:'Image preview'})).toBeVisible()
  await preview.getByRole('button',{name:'Cancel',exact:true}).click()
  expect((await request.get('/api/v1/files/download?path=/long-press.png')).status()).toBe(404)
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  const choose = page.waitForEvent('filechooser')
  await dialog.getByRole('button',{name:'Choose photo',exact:true}).click()
  const chooser = await choose
  await chooser.setFiles([])
  await expect(dialog).toBeVisible()
  const chooseAgain = page.waitForEvent('filechooser')
  await dialog.getByRole('button',{name:'Choose photo',exact:true}).click()
  // Unsupported image preview must retain the original bytes for upload.
  const name = `original-${info.project.name}.heic`
  const original = Buffer.from('unsupported format fixture')
  await (await chooseAgain).setFiles({name,mimeType:'image/heic',buffer:original})
  await expect(preview.getByRole('status')).toContainText('cannot be previewed')
  await preview.getByRole('button',{name:'Upload',exact:true}).click()
  await expect.poll(async () => (await request.get(`/api/v1/files/download?path=/${name}`)).status()).toBe(200)
  expect(await (await request.get(`/api/v1/files/download?path=/${name}`)).body()).toEqual(original)
})

test('wide touch screens keep upload choices while desktop opens its file picker', async ({page}) => {
  await page.setViewportSize({width:1024,height:900})
  await page.goto('/')
  const touch = await page.evaluate(() => matchMedia('(pointer: coarse)').matches)
  const upload = page.getByRole('button',{name:'Upload files',exact:true})
  if (touch) {
    await upload.click()
    const sheet = page.getByRole('dialog',{name:'Upload',exact:true})
    await expect(sheet.getByRole('button',{name:'Photos & videos',exact:true})).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(sheet).not.toBeVisible()
    await expect(upload).toBeFocused()
  } else {
    const choose = page.waitForEvent('filechooser')
    await upload.click()
    await (await choose).setFiles([])
    await expect(page.getByRole('dialog',{name:'Upload',exact:true})).not.toBeVisible()
  }
})
