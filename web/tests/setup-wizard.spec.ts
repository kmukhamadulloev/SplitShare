import {test,expect} from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
test('mandatory wizard retains choices, handles picker cancellation and completes access setup',async({page,request},info)=>{
 const status = await (await request.get('/api/v1/status')).json()
 let required=true, selected=false, cancelled=false, failed=false, finished=false
 let submitted:any
 await page.route('**/api/v1/status',route=>route.fulfill({json:{...status,sharing:!required}}))
 const snapshot=()=>({setup_required:required,folder_selected:selected,bind_ip:'127.0.0.1',port:8080,state:failed?'failed':cancelled?'cancelled':'ready',message:failed?'That port is unavailable.':cancelled?'Folder selection cancelled.':null,local_url:'http://127.0.0.1:43123/',interfaces:[{address:'127.0.0.1',label:'This computer only'},{address:'0.0.0.0',label:'All interfaces'}]})
 await page.route('**/api/v1/host/setup',route=>route.fulfill({json:snapshot()}))
 let picks=0
 await page.route('**/api/v1/host/folder',route=>{picks++;cancelled=picks===1;selected=picks>1;return route.fulfill({status:202,json:{...snapshot(),state:'selecting'}})})
 await page.route('**/api/v1/host/setup/complete',async route=>{
  submitted=route.request().postDataJSON()
  if(!failed) failed=true
  else {failed=false;required=false;finished=true}
  await route.fulfill({status:202,json:{...snapshot(),state:'applying'}})
 })
 await page.goto('/')
 const wizard=page.getByRole('dialog',{name:'Set up SplitShare',exact:true})
 await expect(wizard).toBeVisible()
 await wizard.getByRole('button',{name:'Choose shared folder',exact:true}).click()
 await expect(wizard.getByRole('status')).toContainText('cancelled')
 await expect(wizard.getByRole('button',{name:'Next',exact:true})).toBeDisabled()
 await wizard.getByRole('button',{name:'Choose shared folder',exact:true}).click()
 await expect(wizard.getByText('Folder selected',{exact:true})).toBeVisible()
 await wizard.getByRole('button',{name:'Next',exact:true}).click()
 await expect(wizard.getByLabel('Port',{exact:true})).toHaveValue('8080')
 await wizard.getByLabel('Port',{exact:true}).fill('43123')
 await wizard.getByLabel('Connection',{exact:true}).selectOption('0.0.0.0')
 await page.screenshot({path:`test-results/setup-${info.project.name}.png`,animations:'disabled'})
 expect((await new AxeBuilder({page}).include('.setup-wizard').analyze()).violations).toEqual([])
 await wizard.getByRole('button',{name:'Next',exact:true}).click()
 await expect(wizard.getByLabel('Share access',{exact:true})).toHaveValue('token_link')
 await wizard.getByLabel('Permissions',{exact:true}).selectOption('upload')
 await wizard.getByRole('button',{name:'Start sharing',exact:true}).click()
 await expect(wizard.getByRole('alert')).toContainText('unavailable')
 await wizard.getByRole('button',{name:'Back',exact:true}).click()
 await expect(wizard.getByLabel('Port',{exact:true})).toHaveValue('43123')
 await expect(wizard.getByLabel('Connection',{exact:true})).toHaveValue('0.0.0.0')
 await wizard.getByRole('button',{name:'Next',exact:true}).click()
 await wizard.getByRole('button',{name:'Start sharing',exact:true}).click()
 await expect(wizard).not.toBeVisible()
 await expect(page.getByRole('dialog',{name:'Share with QR',exact:true})).toBeVisible()
 expect(finished).toBe(true)
 expect(submitted.settings.permissions).toEqual({browse:true,download:true,upload:true,create_directory:true,rename:false,delete:false})
 expect(submitted.network).toEqual({bind_ip:'0.0.0.0',port:43123})
})
