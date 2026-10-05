import {test,expect} from '@playwright/test';

test('native Cellar shell retains browsing and quick filters in dark and light palettes',async({page})=>{
  await page.goto('/');await expect(page.getByRole('grid')).toBeVisible();
  await expect(page.locator('.titlebar')).toHaveCSS('height','34px');
  await expect(page.locator('.native-sidebar')).toHaveCSS('width','256px');
  await expect(page.locator('.tabs')).toHaveCSS('height','31px');
  await expect(page.getByRole('row').nth(1)).toHaveCSS('height','25px');
  const grid=await page.getByRole('grid').boundingBox();
  const actions=await page.locator('.result-tabs').boundingBox();
  expect(actions!.y).toBeGreaterThan(grid!.y);
  await page.getByLabel('Quick filter',{exact:true}).fill('Demo customer 001');
  await page.getByLabel('Quick filter',{exact:true}).press('Enter');
  await expect(page.getByRole('gridcell').filter({hasText:'Demo customer 001'})).toBeVisible();
  await expect(page.getByRole('grid')).toHaveAttribute('aria-rowcount','2');
  await page.getByRole('button',{name:'Clear all',exact:true}).click();
  await page.getByRole('button',{name:'Toggle sidebar',exact:true}).click();
  await expect(page.locator('.native-sidebar')).toHaveCount(0);
  await page.getByRole('button',{name:'Toggle sidebar',exact:true}).click();
  await expect(page.locator('.native-sidebar')).toBeVisible();
  await page.screenshot({path:'assets/native-match-dark.png'});
  await page.getByRole('button',{name:'Theme',exact:true}).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme','light');
  await page.screenshot({path:'assets/native-match-light.png'});
});
