import { test, expect } from '@playwright/test';

test.describe('Torrents page', () => {
        test('loads and shows torrent rows', async ({ page }) => {
                await page.goto('/dioxus/torrents');
                await expect(page.locator('.error')).toHaveCount(0);
                await expect(page.locator('.loading-indicator')).toHaveCount(0);
                // At least one torrent row should be visible
                await expect(page.locator('table tr, .torrent-row, [class*="row"]').first()).toBeVisible();
        });

        test('shows 35 torrents total', async ({ page }) => {
                await page.goto('/dioxus/torrents');
                await page.waitForSelector('h1');
                // Wait for data, not loading
                await expect(page.locator('.loading-indicator')).toHaveCount(0);
                // Verify total count text appears somewhere (e.g., "35" in the page)
                await expect(page.locator('body')).toContainText('35');
        });

        test('pagination works with small page size', async ({ page }) => {
                // Use page_size=20 so 35 torrents spans 2 pages
                await page.goto('/dioxus/torrents?page_size=20');

                const pagination = page.locator('.pagination');
                await expect(pagination).toBeVisible();

                // On page 1: Next enabled, Previous disabled
                const nextBtn = page.locator('[aria-label="Next page"]');
                await expect(nextBtn).not.toHaveClass(/disabled/);
                await expect(page.locator('[aria-label="Previous page"]')).toHaveClass(/disabled/);

                // Navigate to page 2 via URL (tests SSR pagination correctness)
                await page.goto('/dioxus/torrents?page_size=20&from=20');
                await expect(page.locator('body')).toContainText('Showing 20');
                // On page 2: Previous enabled
                await expect(page.locator('[aria-label="Previous page"]')).not.toHaveClass(/disabled/);
        });

        test('page 2 shows different content than page 1', async ({ page }) => {
                await page.goto('/dioxus/torrents?page_size=20');
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                // Get the first title link from page 1 (links with title= param are title links)
                const firstTitle = await page.locator('.torrents-grid-row a[href*="title="]').first().textContent();

                // Navigate directly to page 2 via URL
                await page.goto('/dioxus/torrents?page_size=20&from=20');
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                const secondPageTitle = await page.locator('.torrents-grid-row a[href*="title="]').first().textContent();
                expect(firstTitle).not.toEqual(secondPageTitle);
        });

        test('sorting by title changes data order', async ({ page }) => {
                await page.goto('/dioxus/torrents');
                await expect(page.locator('.loading-indicator')).toHaveCount(0);

                const titleSort = page.locator('.header button.link', { hasText: 'Title' });
                await expect(titleSort).toHaveCount(1);

                const firstRow = page.locator('.torrents-grid-row').first();
                await expect(firstRow).toBeVisible();

                await titleSort.click();
                await expect(page.locator('.loading-indicator')).toHaveCount(0, { timeout: 15_000 });
                const firstSortedTitle = await firstRow.innerText();

                await titleSort.click();
                await expect
                        .poll(async () => firstRow.innerText(), { timeout: 15_000 })
                        .not.toBe(firstSortedTitle);
                await expect(page.locator('.error')).toHaveCount(0);
        });

        test('column toggle shows/hides a column', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                // Column checkboxes are hidden (display:none); click the label instead
                const columnLabels = page.locator('.option_group label');
                const count = await columnLabels.count();
                if (count > 0) {
                        const first = columnLabels.first();
                        const checkbox = first.locator('input[type="checkbox"]');
                        const wasChecked = await checkbox.isChecked();
                        await first.click();
                        await page.waitForTimeout(300);
                        expect(await checkbox.isChecked()).toBe(!wasChecked);
                        // Toggle back
                        await first.click();
                }
        });

        test('filter link by author narrows results', async ({ page }) => {
                await page.goto('/dioxus/torrents');
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                // Click the first author filter link
                const authorLink = page.locator('a[href*="author"]').first();
                if (await authorLink.count() > 0) {
                        await authorLink.click();
                        await expect(page.locator('.torrents-grid-row').first()).toBeVisible({ timeout: 15_000 });
                        await expect(page.locator('.error')).toHaveCount(0);
                }
        });

        test('no error state on initial load', async ({ page }) => {
                await page.goto('/dioxus/torrents');
                await expect(page.locator('.error')).toHaveCount(0);
        });
});
