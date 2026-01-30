import { test, expect } from '@playwright/test';
import fc from 'fast-check';

/**
 * Property 1: Responsive Layout Integrity
 *
 * *For any* viewport width between 320px and 2560px, the landing page
 * SHALL render without horizontal scrollbar overflow.
 *
 * **Validates: Requirements 8.1**
 * **Feature: caliber-landing-page, Property 1: Responsive Layout Integrity**
 */
test('Property 1: No horizontal overflow at any viewport width', async ({ page }) => {
  test.setTimeout(180000); // 3 minutes for 100 iterations

  await fc.assert(
    fc.asyncProperty(fc.integer({ min: 320, max: 2560 }), async (viewportWidth) => {
      await page.setViewportSize({ width: viewportWidth, height: 800 });
      await page.goto('/');

      // Wait for page to stabilize
      await page.waitForLoadState('networkidle');

      const hasHorizontalScroll = await page.evaluate(() => {
        return document.documentElement.scrollWidth > document.documentElement.clientWidth;
      });

      expect(hasHorizontalScroll).toBe(false);
    }),
    { numRuns: 100 }
  );
});
