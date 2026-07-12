import { test, expect } from '@playwright/test';

test('renders the home page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Sigma Identity' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Sign in', exact: true })).toBeVisible();
});

test('hides the admin section from an anonymous visitor', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Users' })).toHaveCount(0);
});

test('shows register/sign-in for an anonymous visitor', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Register / Sign in' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Update profile' })).toHaveCount(0);
});
