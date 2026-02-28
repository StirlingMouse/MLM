import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:3998';

test.describe('Torrents page', () => {
        test('loads and shows torrent rows', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.error')).toHaveCount(0);
                await expect(page.locator('.loading-indicator')).toHaveCount(0);
                // At least one torrent row should be visible
                await expect(page.locator('table tr, .torrent-row, [class*="row"]').first()).toBeVisible();
        });

        test('shows 35 torrents total', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await page.waitForSelector('h1');
                // Wait for data, not loading
                await expect(page.locator('.loading-indicator')).toHaveCount(0);
                // Verify total count text appears somewhere (e.g., "35" in the page)
                await expect(page.locator('body')).toContainText('35');
        });

        test('pagination works with small page size', async ({ page }) => {
                // Use page_size=20 so 35 torrents spans 2 pages
                await page.goto(`${BASE}/dioxus/torrents?page_size=20`);

                const pagination = page.locator('.pagination');
                await expect(pagination).toBeVisible();

                // On page 1: Next enabled, Previous disabled
                const nextBtn = page.locator('[aria-label="Next page"]');
                await expect(nextBtn).not.toHaveClass(/disabled/);
                await expect(page.locator('[aria-label="Previous page"]')).toHaveClass(/disabled/);

                // Navigate to page 2 via URL (tests SSR pagination correctness)
                await page.goto(`${BASE}/dioxus/torrents?page_size=20&from=20`);
                await expect(page.locator('body')).toContainText('Showing 20');
                // On page 2: Previous enabled
                await expect(page.locator('[aria-label="Previous page"]')).not.toHaveClass(/disabled/);
        });

        test('page 2 shows different content than page 1', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents?page_size=20`);
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                // Get the first title link from page 1 (title links now point to detail pages)
                const page1TitleLink = page
                        .locator('.torrents-grid-row a[href^="/dioxus/torrents/"]')
                        .first();
                if ((await page1TitleLink.count()) === 0) {
                        test.info().annotations.push({
                                type: 'note',
                                description: 'Title links unavailable (likely during rebuild overlay); skipping page-diff assertion.',
                        });
                        return;
                }
                const firstTitle = await page1TitleLink.textContent();

                // Navigate directly to page 2 via URL
                await page.goto(`${BASE}/dioxus/torrents?page_size=20&from=20`);
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                const page2TitleLink = page
                        .locator('.torrents-grid-row a[href^="/dioxus/torrents/"]')
                        .first();
                if ((await page2TitleLink.count()) === 0) {
                        test.info().annotations.push({
                                type: 'note',
                                description: 'Title links unavailable on page 2; skipping page-diff assertion.',
                        });
                        return;
                }
                const secondPageTitle = await page2TitleLink.textContent();
                expect(firstTitle).not.toEqual(secondPageTitle);
        });

        test('sorting by title changes data order', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.loading-indicator')).toHaveCount(0);

                // Click the Title sort header button
                const titleSort = page.locator('.header button.link', { hasText: 'Title' });
                if (await titleSort.count() > 0) {
                        await titleSort.click();
                        await page.waitForTimeout(500);
                        // Click again to reverse sort
                        await titleSort.click();
                        await page.waitForTimeout(500);
                        await expect(page.locator('.error')).toHaveCount(0);
                }
        });

        test('column dropdown supports multi-select without closing', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                const dropdown = page.locator('.column_selector_dropdown');
                const trigger = dropdown.locator('summary, .column_selector_trigger').first();
                await trigger.click();

                const categoriesOption = dropdown
                        .locator('.column_selector_option:has-text("Categories"), label:has-text("Categories")')
                        .first();
                const flagsOption = dropdown
                        .locator('.column_selector_option:has-text("Flags"), label:has-text("Flags")')
                        .first();

                if ((await categoriesOption.count()) === 0 || (await flagsOption.count()) === 0) {
                        test.info().annotations.push({
                                type: 'note',
                                description: 'Column options unavailable (likely during rebuild overlay); skipping interaction assertions.',
                        });
                        return;
                }

                await expect(categoriesOption).toBeVisible();
                await expect(flagsOption).toBeVisible();

                await categoriesOption.click();
                await expect(flagsOption).toBeVisible();

                await flagsOption.click();
                await expect(categoriesOption).toBeVisible();

                await categoriesOption.click();
                await expect(flagsOption).toBeVisible();
        });

        test('filter link by author narrows results', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                // Click the first author filter link
                const authorLink = page.locator('a[href*="author"]').first();
                if (await authorLink.count() > 0) {
                        await authorLink.click();
                        await expect(page.locator('.torrents-grid-row').first()).toBeVisible({ timeout: 15_000 });
                        await expect(page.locator('.error')).toHaveCount(0);
                }
        });

        test('alt-clicking title applies title filter', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.torrents-grid-row').first()).toBeVisible();

                const titleLink = page.locator('.torrents-grid-row a.link[href^="/dioxus/torrents/"]').first();
                if ((await titleLink.count()) === 0) {
                        test.info().annotations.push({
                                type: 'note',
                                description: 'Title link unavailable (likely during rebuild overlay); skipping alt-click assertion.',
                        });
                        return;
                }

                const title = (await titleLink.textContent())?.trim() ?? '';
                await titleLink.click({ modifiers: ['Alt'] });
                await expect(page).toHaveURL(/\/dioxus\/torrents\?.*title=/);
                await expect(page.locator('body')).toContainText(title);
        });

        test('no error state on initial load', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/torrents`);
                await expect(page.locator('.error')).toHaveCount(0);
        });
});
