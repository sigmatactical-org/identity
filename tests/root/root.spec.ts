import { test, expect } from '@playwright/test';
import { configuration } from '../conf.ts';

test('test', async ({ page }) => {
  const conf = await configuration();
  await page.goto(conf.baseUrl);
  await expect(page.getByRole('heading', { name: 'Sigma Tactical Group' })).toBeVisible();
});
