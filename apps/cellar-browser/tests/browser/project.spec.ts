import {test,expect} from '@playwright/test';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('../../.fixtures/project-demo',import.meta.url));

async function discover(page:import('@playwright/test').Page){
 await page.goto('/');await expect(page.getByRole('grid')).toBeVisible();
 await page.getByRole('button',{name:'Connect from this project',exact:true}).click();
 const dialog=page.getByRole('dialog',{name:'Connect from this project'});
 await expect(dialog.getByRole('button',{name:'Use current Codex project'})).toBeDisabled();
 await dialog.getByLabel('Project folder',{exact:true}).fill(root);
 await dialog.getByRole('button',{name:'Use this folder',exact:true}).click();
 await expect(dialog.getByRole('button',{name:'Discover connections',exact:true})).toBeDisabled();
 await dialog.getByLabel('Allow local project configuration discovery').check();
 await dialog.getByRole('button',{name:'Discover connections',exact:true}).click();
 await expect(dialog.getByRole('group',{name:'Choose a connection'})).toBeVisible();return dialog;
}
test('project discovery requires local consent, hides credentials and connects selected SQLite read-only',async({page})=>{
 const dialog=await discover(page);
 await expect(dialog).not.toContainText('synthetic-password');await expect(dialog).not.toContainText('cellar_reader');await expect(dialog).not.toContainText('postgresql://');
 await expect(dialog).toContainText('unresolved variables were skipped');
 await dialog.getByRole('radio',{name:/Select sqlite from data\/demo.sqlite/}).check();
 await expect(dialog.getByRole('button',{name:'Connect read-only',exact:true})).toBeDisabled();
 await dialog.getByLabel('Confirm read-only database access').check();await dialog.getByRole('button',{name:'Connect read-only',exact:true}).click();
 await expect(dialog).toHaveCount(0);await expect(page.getByRole('grid')).toBeVisible();
 await expect(page.locator('.title-crumbs')).toContainText('Project · SQLite');
 await expect(page.locator('footer')).toContainText('READ ONLY');
 await expect(page.getByRole('button',{name:'Download database',exact:true})).toHaveCount(0);
 await page.getByRole('button',{name:/▦ orders/}).click();await expect(page.getByRole('gridcell').filter({hasText:'synthetic'}).first()).toBeVisible();
 await page.getByRole('button',{name:'Connect from this project',exact:true}).click();
 await page.getByRole('dialog',{name:'Connect from this project'}).getByRole('button',{name:'Forget project connections',exact:true}).click();
 await expect(page.locator('.title-crumbs')).toContainText('Cellar demo');
 await expect(page.getByLabel('Connection').locator('option').filter({hasText:'Project ·'})).toHaveCount(0);
});
test('production-like project target needs extra acknowledgement and cancel does not connect',async({page})=>{
 const dialog=await discover(page);
 await dialog.getByRole('radio',{name:/Select postgres from .env.production/}).check();
 await dialog.getByLabel('Confirm read-only database access').check();await expect(dialog.getByRole('button',{name:'Connect read-only',exact:true})).toBeDisabled();
 await dialog.getByLabel('Acknowledge production-like target').check();await expect(dialog.getByRole('button',{name:'Connect read-only',exact:true})).toBeEnabled();
 await dialog.getByRole('button',{name:'Cancel',exact:true}).click();await expect(page.locator('.title-crumbs')).toContainText('Cellar demo');
});
test('project PostgreSQL connection uses only the disposable fixture',async({page})=>{
 test.skip(process.env.CELLAR_TEST_POSTGRES!=='1','Requires the disposable PostgreSQL fixture');
 const dialog=await discover(page);await dialog.getByRole('radio',{name:/Select postgres from .env \d/}).check();
 await dialog.getByLabel('Confirm read-only database access').check();await dialog.getByRole('button',{name:'Connect read-only',exact:true}).click();
 await expect(page.locator('.title-crumbs')).toContainText('Project · PostgreSQL');
 await page.getByRole('button',{name:/▦ orders/}).click();await expect(page.getByRole('gridcell').filter({hasText:'synthetic'}).first()).toBeVisible();
 await expect(page.locator('footer')).toContainText('READ ONLY');
});
