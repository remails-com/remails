import { test, expect } from '../playwright/fixtures';

test('basic walk though', async ({ page }) => {
  await page.goto('http://localhost:3000/');

  await expect(page.getByRole('button', { name: 'New Project' })).toBeVisible();

  await page.locator('a').filter({ hasText: 'Domains' }).click();
  await expect(page.getByRole('main')).toContainText('domains');

  await page.locator('a').filter({ hasText: 'Statistics' }).click();
  await expect(page.getByRole('main')).toContainText('statistics');

});

test('rename organization', async ({ page }) => {
  await page.goto('http://localhost:3000/');

  await page.locator('a').filter({ hasText: 'Settings' }).click();
  await expect(page.getByRole('tabpanel', { name: 'Subscription' })).toBeVisible();

  // rename organization
  await page.getByRole('heading').filter({hasNotText: 'Your subscription'}).click();
  await page.getByRole('textbox').fill('renamed organization');
  await expect(page.locator('.tabler-icon.tabler-icon-check')).toBeVisible();
  await page.locator('.tabler-icon.tabler-icon-check').click();
  await expect(page.getByRole('main')).toContainText('renamed organization');
})

test('organization invite', async ({ page }) => {
  await page.goto('http://localhost:3000/');

  await page.locator('a').filter({ hasText: 'Settings' }).click();
  await expect(page.getByRole('tabpanel', { name: 'Subscription' })).toBeVisible();

  await page.getByRole('tab', { name: 'Members' }).click();
  await expect(page.getByRole('heading', { name: 'Organization members' })).toBeVisible();

  await page.getByRole('button', { name: 'New invite link' }).click();
  await expect(page.getByRole('dialog', { name: 'Create new invite link' })).toBeVisible();

  await page.getByRole('textbox', { name: 'Organization role' }).click();
  await expect(page.getByRole('listbox', { name: 'Organization role' })).toBeVisible();

  await page.getByRole('option', { name: 'Maintainer' }).click();
  await page.getByRole('button', { name: 'Create', exact: true }).click();

  await page.getByRole('button', { name: 'Done' }).click();
  await expect(page.getByRole('heading', { name: 'Organization invites' })).toBeVisible();

  await page.locator('.tabler-icon.tabler-icon-trash').click();
  await expect(page.getByRole('dialog', { name: 'Please confirm your action' })).toBeVisible();

  await page.getByRole('button', { name: 'Confirm' }).click();

  await expect(page.getByLabel('Members')).not.toContainText('Organization invites');

})

test('organization API key', async ({ page }) => {
  await page.goto('http://localhost:3000/');

  await page.locator('a').filter({ hasText: 'Settings' }).click();
  await expect(page.getByRole('tabpanel', { name: 'Subscription' })).toBeVisible();


  await page.getByRole('tab', { name: 'API Keys' }).click();
  await expect(page.getByRole('button', { name: 'New API Key' })).toBeVisible();

})