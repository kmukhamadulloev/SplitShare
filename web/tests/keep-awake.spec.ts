import {test,expect,type Page} from '@playwright/test'
import {networkInterfaces} from 'node:os'
async function pendingUpload(page: Page, url = '/') {
  let finish!: () => void
  const gate = new Promise<void>(resolve => { finish = resolve })
  await page.route('**/api/v1/uploads?**',async route => {await gate; await route.continue()})
  await page.goto(url)
  await page.locator('input[type=file]').setInputFiles({name:`awake-${test.info().project.name}-${test.info().title.length}.txt`,mimeType:'text/plain',buffer:Buffer.from('awake test')})
  await expect(page.getByRole('button',{name:'Keep screen awake',exact:true})).toBeVisible()
  return finish
}

test('video fallback is local, silent, opt-in and stops when the batch completes',async ({page}) => {
  await page.addInitScript(() => Object.defineProperty(navigator,'wakeLock',{value:undefined}))
  // Force HTTP fallback even though the test origin is localhost.
  await page.addInitScript(() => Object.defineProperty(window,'isSecureContext',{value:false}))
  const finish = await pendingUpload(page)
  const video = page.locator('.keep-awake-video')
  expect(await video.evaluate((v: HTMLVideoElement) => v.paused)).toBe(true)
  await page.getByRole('button',{name:'Keep screen awake',exact:true}).click()
  await expect(page.getByText('Keep-awake fallback running. Your device may still lock.',{exact:true})).toBeVisible()
  expect(await video.evaluate((v: HTMLVideoElement) => ({paused:v.paused,muted:v.muted,loop:v.loop,inline:v.hasAttribute('playsinline')}))).toEqual({paused:false,muted:true,loop:true,inline:true})
  expect(await video.evaluate((v: HTMLVideoElement) => new URL(v.currentSrc).origin === location.origin)).toBe(true)
  await page.getByRole('button',{name:'Stop keeping awake',exact:true}).click()
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.paused)).toBe(true)
  await page.getByRole('button',{name:'Keep screen awake',exact:true}).click()
  await expect(page.getByText('Keep-awake fallback running. Your device may still lock.',{exact:true})).toBeVisible()
  finish()
  await expect(page.getByRole('button',{name:'Stop keeping awake',exact:true})).toHaveCount(0)
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.paused)).toBe(true)
})

test('native wake lock releases on page hide and completion without automatic reactivation',async ({page}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator,'wakeLock',{value:{request:async () => {
      const lock = new EventTarget() as EventTarget & {release:()=>Promise<void>}
      lock.release = async () => {document.documentElement.dataset.released='true'; lock.dispatchEvent(new Event('release'))}
      return lock
    }}})
  })
  const finish = await pendingUpload(page)
  await page.getByRole('button',{name:'Keep screen awake',exact:true}).click()
  await expect(page.getByText('Screen wake lock active. Keep this page open.',{exact:true})).toBeVisible()
  await page.evaluate(() => {Object.defineProperty(document,'hidden',{configurable:true,value:true}); document.dispatchEvent(new Event('visibilitychange'))})
  await expect(page.locator('html')).toHaveAttribute('data-released','true')
  await page.evaluate(() => {Object.defineProperty(document,'hidden',{configurable:true,value:false}); document.dispatchEvent(new Event('visibilitychange'))})
  await expect(page.getByText('Keep-awake stopped. Tap to enable it again.',{exact:true})).toBeVisible()
  await page.getByRole('button',{name:'Keep screen awake',exact:true}).click()
  await expect(page.getByText('Screen wake lock active. Keep this page open.',{exact:true})).toBeVisible()
  await page.locator('html').evaluate(el=>el.removeAttribute('data-released'))
  finish()
  await expect(page.locator('html')).toHaveAttribute('data-released','true')
})

test('rejected requests and playback failures do not claim a wake lock',async ({page}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator,'wakeLock',{value:{request:async () => {throw new Error('Denied')}}})
    HTMLMediaElement.prototype.play = () => Promise.reject(new Error('Playback blocked'))
  })
  const finish = await pendingUpload(page)
  await page.getByRole('button',{name:'Keep screen awake',exact:true}).click()
  await expect(page.getByText('Could not keep the screen awake.',{exact:false})).toBeVisible()
  await page.getByRole('button',{name:'Try keeping awake',exact:true}).click()
  await expect(page.getByRole('button',{name:'Try keeping awake',exact:true})).toHaveAttribute('aria-pressed','false')
  await expect(page.getByText('Keep-awake fallback running.',{exact:false})).toHaveCount(0)
  finish()
})


test('video fallback plays on an actual insecure HTTP LAN origin',async ({browser,browserName}) => {
  const address = Object.values(networkInterfaces()).flat().find(item => item && item.family === 'IPv4' && !item.internal)?.address
  test.skip(!address,'Requires a LAN interface')
  const directBrowser = browserName === 'firefox' ? await browser.browserType().launch({firefoxUserPrefs:{'network.proxy.type':0}}) : undefined
  const context = await (directBrowser ?? browser).newContext(directBrowser ? {} : {proxy:{server:'http://127.0.0.1:9',bypass:address!}})
  let finish = () => {}
  try {
    const page = await context.newPage()
    finish = await pendingUpload(page,`http://${address}:43123/`)
    expect(await page.evaluate(() => window.isSecureContext)).toBe(false)
    await page.getByRole('button',{name:'Keep screen awake',exact:true}).click()
    await expect(page.getByText('Keep-awake fallback running. Your device may still lock.',{exact:true})).toBeVisible()
    await page.screenshot({path:`test-results/keep-awake-${test.info().project.name}.png`})
    finish()
    await expect(page.getByRole('button',{name:'Stop keeping awake',exact:true})).toHaveCount(0)
    await expect.poll(() => page.locator('video').evaluate((v: HTMLVideoElement)=>v.paused)).toBe(true)
  } finally {finish(); await context.close(); await directBrowser?.close()}
})
