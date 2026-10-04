import {test,expect} from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import {randomBytes} from 'node:crypto'
import {readFile} from 'node:fs/promises'
test('safe text, images, unsupported files and bounded previews',async({page,request},info)=>{
 const prefix=`preview-${info.project.name}`
 const upload=async(name:string,data:Buffer)=>expect((await request.post(`/api/v1/uploads?path=/${prefix}-${name}`,{headers:{'X-SplitShare-Request':'1','Content-Type':'application/octet-stream','X-Transfer-ID':randomBytes(16).toString('hex'),'X-Transfer-Key':randomBytes(16).toString('hex')},data})).status()).toBe(201)
 await upload('page.html',Buffer.from('<script>window.bad=true</script>\nHello preview'))
 await upload('large.txt',Buffer.alloc(300*1024,65))
 await upload('empty.txt',Buffer.alloc(0))
 await upload('binary.txt',Buffer.from([0,1,2,3]))
 await upload('archive.zip',Buffer.from('unsupported'))
 await upload('a.png',await readFile('public/logo.png'))
 await upload('b.png',await readFile('public/logo.png'))
 await page.goto('/')
 await page.getByRole('textbox',{name:'Search files'}).fill(prefix)
 const dialog=page.getByRole('dialog',{name:'File preview',exact:true})
 const open=async(name:string)=>{
  const row=page.getByRole('listitem',{name:`${prefix}-${name}`,exact:true})
  await row.getByRole('button',{name:`Actions for ${prefix}-${name}`,exact:true}).click()
  await page.getByRole('menuitem',{name:'Open',exact:true}).click()
 }
 await open('page.html')
 await expect(dialog.locator('pre')).toContainText('<script>window.bad=true</script>')
 expect(await page.evaluate(()=>Reflect.get(window,'bad'))).toBeUndefined()
 await page.screenshot({path:`test-results/preview-text-${info.project.name}.png`})
 expect((await new AxeBuilder({page}).include('.preview-dialog').analyze()).violations).toEqual([])
 await page.keyboard.press('Escape')
 await open('large.txt')
 await expect(dialog.getByRole('status')).toContainText('first 256 KiB')
 expect((await dialog.locator('pre').innerText()).length).toBe(256*1024)
 await page.keyboard.press('Escape')
 await open('empty.txt'); await expect(dialog.locator('pre')).toHaveText('Empty file.'); await page.keyboard.press('Escape')
 await open('binary.txt'); await expect(dialog.getByRole('alert')).toContainText('not supported UTF-8'); await page.keyboard.press('Escape')
 await open('archive.zip'); await expect(dialog.getByRole('alert')).toContainText('Preview unavailable'); await expect(dialog.getByRole('link',{name:'Download',exact:true})).toBeVisible(); await page.keyboard.press('Escape')
 const imageButton=page.getByRole('button',{name:`${prefix}-a.png`,exact:true})
 if(info.project.name==='mobile') await imageButton.tap(); else await imageButton.dblclick()
 await expect(dialog.locator('img')).toBeVisible()
 await expect(dialog.getByRole('status')).toHaveCount(0)
 await page.screenshot({path:`test-results/preview-image-${info.project.name}.png`})
 await dialog.getByRole('button',{name:'Zoom in',exact:true}).click()
 await page.keyboard.press('ArrowRight')
 await expect(dialog.getByRole('heading')).toHaveText(`${prefix}-b.png`)
 await dialog.getByRole('button',{name:'Close preview',exact:true}).click()
 await expect(imageButton).toBeFocused()
})
test('native audio and video stream and stop on close',async({page,request},info)=>{
 const prefix=`media-${info.project.name}`
 const wav=Buffer.alloc(44+16000)
 wav.write('RIFF'); wav.writeUInt32LE(wav.length-8,4); wav.write('WAVEfmt ',8); wav.writeUInt32LE(16,16); wav.writeUInt16LE(1,20); wav.writeUInt16LE(1,22); wav.writeUInt32LE(8000,24); wav.writeUInt32LE(16000,28); wav.writeUInt16LE(2,32); wav.writeUInt16LE(16,34); wav.write('data',36); wav.writeUInt32LE(16000,40)
 for(const [ext,data] of [['wav',wav],['mp4',await readFile('public/media/keep-awake.mp4')]] as const){
  expect((await request.post(`/api/v1/uploads?path=/${prefix}.${ext}`,{headers:{'X-SplitShare-Request':'1','Content-Type':'application/octet-stream','X-Transfer-ID':randomBytes(16).toString('hex'),'X-Transfer-Key':randomBytes(16).toString('hex')},data})).status()).toBe(201)
 }
 await page.goto('/')
 for(const ext of ['wav','mp4']) {
  await page.getByRole('button',{name:`${prefix}.${ext}`,exact:true}).focus()
  await page.keyboard.press('Enter')
  const dialog=page.getByRole('dialog',{name:'File preview',exact:true})
  const media=dialog.locator(ext==='wav'?'audio':'video')
  await expect.poll(()=>media.evaluate((el:HTMLMediaElement)=>el.readyState)).toBeGreaterThan(0)
  await media.evaluate(async(el:HTMLMediaElement)=>{await el.play()})
  await expect.poll(()=>media.evaluate((el:HTMLMediaElement)=>el.currentTime)).toBeGreaterThan(0)
  await page.keyboard.press('Escape')
  await expect(dialog).not.toBeVisible()
  await expect(page.locator('.preview-dialog video,.preview-dialog audio')).toHaveCount(0)
 }
})
