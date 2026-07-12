import { test, expect } from '@playwright/test';

test('renders the home page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Sigma Identity' })).toBeVisible();
  await expect(page.locator('#store-nav-signed-out').getByRole('link', { name: 'Sign in' })).toBeVisible();
});

test('hides the admin section from an anonymous visitor', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Users' })).toHaveCount(0);
});

test('shows sign-in above register for an anonymous visitor', async ({ page }) => {
  await page.goto('/');
  const account = page.getByRole('navigation', { name: 'Account' });
  await expect(account.getByRole('link', { name: 'Sign in' })).toBeVisible();
  await expect(account.getByRole('link', { name: 'Register' })).toBeVisible();
  await expect(account.getByRole('link', { name: 'Update profile' })).toHaveCount(0);

  const itemNames = await account.getByRole('listitem').allTextContents();
  const signInIndex = itemNames.findIndex((t) => t.includes('Sign in'));
  const registerIndex = itemNames.findIndex((t) => t.includes('Register'));
  expect(signInIndex).toBeGreaterThanOrEqual(0);
  expect(signInIndex).toBeLessThan(registerIndex);
});
