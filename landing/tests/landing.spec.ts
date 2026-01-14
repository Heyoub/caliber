import { test, expect } from '@playwright/test';

test.describe('Landing Page', () => {
  test('page loads successfully', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/CALIBER/);
  });

  test('hero section displays correctly', async ({ page }) => {
    await page.goto('/');
    const hero = page.locator('section').first();
    await expect(hero).toBeVisible();
    await expect(page.getByText('AI agents forget everything')).toBeVisible();
  });

  test('navigation links are present', async ({ page }) => {
    await page.goto('/');
    const nav = page.locator('nav').first();
    await expect(nav).toBeVisible();
    await expect(nav.getByRole('link', { name: /problem/i })).toBeVisible();
    await expect(nav.getByRole('link', { name: /solution/i })).toBeVisible();
    await expect(nav.getByRole('link', { name: /pricing/i })).toBeVisible();
  });

  test('pricing section displays correct values', async ({ page }) => {
    await page.goto('/');
    const pricing = page.locator('#pricing');
    await expect(pricing).toBeVisible();
    // Check storage pricing ($1/mo)
    await expect(pricing.getByText('$1', { exact: false }).first()).toBeVisible();
    // Check hot cache pricing ($0.15/mo)
    await expect(pricing.getByText('$0.15', { exact: false })).toBeVisible();
    // Check unlimited agents
    await expect(pricing.getByText('unlimited')).toBeVisible();
  });

  test('footer contains required links', async ({ page }) => {
    await page.goto('/');
    const footer = page.locator('footer');
    await expect(footer).toBeVisible();
    await expect(footer.getByLabel('GitHub')).toBeVisible();
    // Check for AGPL-3.0 license link
    await expect(footer.getByRole('link', { name: 'AGPL-3.0', exact: true })).toBeVisible();
  });
});

test.describe('Mobile Navigation', () => {
  test.use({ viewport: { width: 375, height: 667 } });

  test('mobile menu toggles correctly', async ({ page }) => {
    await page.goto('/');
    const menuButton = page.getByRole('button', { name: /menu/i });
    await expect(menuButton).toBeVisible();
    await menuButton.click();
    const mobileMenu = page.locator('[data-mobile-menu]');
    await expect(mobileMenu).toBeVisible();
  });
});
