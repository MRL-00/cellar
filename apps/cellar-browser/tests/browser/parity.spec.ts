import {test,expect} from '@playwright/test';

test('connection folders persist, collapse and retain browsing',async({page})=>{
  await page.goto('/');await expect(page.getByRole('grid')).toBeVisible();
  await page.getByRole('button',{name:'Connection actions',exact:true}).click();
  await page.getByRole('menuitem',{name:'Organize connections…',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'Organize connections'});
  await dialog.getByLabel('Folder name',{exact:true}).fill('Testing');
  await dialog.getByRole('button',{name:'New folder',exact:true}).click();
  const select=dialog.getByLabel('Folder for Cellar demo · SQLite',{exact:true});
  const group=await select.locator('option').filter({hasText:'Testing'}).getAttribute('value');
  await select.selectOption(group!);await dialog.getByRole('button',{name:'Done',exact:true}).click();
  const folder=page.locator('.folder-row').filter({hasText:'Testing'});
  await expect(folder).toHaveAttribute('aria-expanded','true');
  await folder.click();await expect(folder).toHaveAttribute('aria-expanded','false');
  await expect(page.getByRole('button',{name:/▦ orders/})).toHaveCount(0);
  await page.reload();await expect(page.getByRole('grid')).toBeVisible();
  await expect(page.locator('.folder-row').filter({hasText:'Testing'})).toHaveAttribute('aria-expanded','false');
  await page.locator('.folder-row').filter({hasText:'Testing'}).click();
  await page.getByRole('button',{name:/▦ orders/}).click();
  await expect(page.getByRole('gridcell').filter({hasText:'synthetic'}).first()).toBeVisible();
});

test('table presets restore filters and descending sort after reload and stay table-scoped',async({page})=>{
  await page.goto('/');await expect(page.getByRole('grid')).toBeVisible();
  await page.getByRole('button',{name:/▦ orders/}).click();
  await page.getByRole('button',{name:'where',exact:true}).click();
  await page.getByLabel('Filter column',{exact:true}).selectOption('status');await page.getByLabel('Filter operator').selectOption('equals');await page.getByLabel('Filter value').fill('paid');
  await page.getByRole('button',{name:'＋ Filter',exact:true}).click();
  await expect(page.getByRole('button',{name:'status equals paid ×',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Done',exact:true}).click();
  await page.getByLabel('Sort column').selectOption('id');await expect(page.getByRole('gridcell',{name:'3',exact:true}).first()).toBeVisible();
  await page.getByRole('button',{name:'Sort direction',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'348',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Presets',exact:true}).click();await page.getByRole('menuitem',{name:'Save current as preset…'}).click();
  await page.getByLabel('Preset name').fill('Paid newest');await page.getByRole('button',{name:'Save preset',exact:true}).click();
  await page.reload();await expect(page.getByRole('grid')).toBeVisible();await page.getByRole('button',{name:/▦ orders/}).click();
  await page.getByRole('button',{name:'Presets',exact:true}).click();await page.getByRole('menuitem',{name:'Paid newest',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'348',exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'status equals paid ×',exact:true})).toBeVisible();
  await page.getByRole('button',{name:/▦ customers/}).click();await page.getByRole('button',{name:'Presets',exact:true}).click();
  await expect(page.getByRole('menuitem',{name:'Paid newest',exact:true})).toHaveCount(0);
});

test('execution panel shows real warnings, errors, EXPLAIN and history; workspace delegates AI to Codex',async({page})=>{
  const errors:string[]=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto('/');await expect(page.getByRole('grid')).toBeVisible();await page.getByRole('button',{name:/▦ orders/}).click();
  await expect(page.getByRole('gridcell').filter({hasText:'synthetic'}).first()).toBeVisible();
  await page.getByRole('tab',{name:/^Messages/}).click();
  await expect(page.getByRole('tabpanel',{name:'Messages panel'})).toContainText('Response hit the 100-row or byte limit');
  await page.getByRole('button',{name:'warning 1',exact:true}).click();await expect(page.locator('.execution-messages tbody tr')).toHaveCount(1);
  await page.getByRole('tab',{name:/^Notices/}).click();await expect(page.getByRole('tabpanel',{name:'Notices panel'})).toContainText('more rows may exist');
  await page.getByRole('button',{name:'＋ New SQL query',exact:true}).click();
  const editor=page.locator('.cm-content');await editor.click();await page.keyboard.press('Meta+a');await page.keyboard.insertText('SELECT * FROM orders WHERE id = 1');
  await page.getByRole('button',{name:'▶ Run query',exact:true}).click();await expect(page.getByRole('gridcell',{name:'1',exact:true}).first()).toBeVisible();
  await page.getByRole('tab',{name:'Plan',exact:true}).click();await expect(page.getByRole('tabpanel',{name:'Plan panel'})).toContainText('SEARCH');
  await page.getByRole('tab',{name:/^History/}).click();await expect(page.getByRole('tabpanel',{name:'History panel'})).toContainText('SELECT * FROM orders WHERE id = 1');
  await editor.click();await page.keyboard.press('Meta+a');await page.keyboard.insertText('DELETE FROM orders');await page.getByRole('button',{name:'▶ Run query',exact:true}).click();await expect(page.getByRole('alert')).toContainText('read-only SELECT');
  await page.getByRole('tab',{name:/^Messages/}).click();await expect(page.getByRole('tabpanel',{name:'Messages panel'})).toContainText('read-only SELECT');
  await expect(page.getByRole('button',{name:'Toggle AI Assistant',exact:true})).toHaveCount(0);
  await expect(page.getByRole('complementary',{name:'AI Assistant'})).toHaveCount(0);
  expect(errors).toEqual([]);
});
