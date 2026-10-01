import {test,expect} from '@playwright/test'
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aSuoAAAAASUVORK5CYII='

test('image paste works inside clipboard permission fallback and preserves bytes', async ({page,request},info) => {
  await page.addInitScript(() => Object.defineProperty(navigator,'clipboard',{value:{read:async () => {throw new Error('Denied')}}}))
  await page.goto('/')
  await page.getByRole('button',{name:'Paste',exact:true}).click()
  await expect(page.getByRole('alert')).toContainText('Clipboard access is unavailable')
  await page.getByLabel('Text',{exact:true}).evaluate((element,png) => {
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
