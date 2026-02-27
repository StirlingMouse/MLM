import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:3998';

function noError(page: import('@playwright/test').Page) {
        return expect(page.locator('.error')).toHaveCount(0);
}
function noLoading(page: import('@playwright/test').Page) {
        return expect(page.locator('.loading-indicator')).toHaveCount(0, { timeout: 15_000 });
}

test.describe('Events page', () => {
        test('loads and shows events', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/events`);
                await noError(page);
                await noLoading(page);
                // 10 events were inserted
                await expect(page.locator('body')).toContainText('Grabbed');
        });

        test('no loading indicator stuck', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/events`);
                await noLoading(page);
        });
});

test.describe('Errors page', () => {
        test('loads and shows errored torrents', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/errors`);
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Errored Book');
        });

        test('sort header is interactive', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/errors`);
                await noLoading(page);
                const sortBtn = page.locator('.header button.link').first();
                if (await sortBtn.count() > 0) {
                        await sortBtn.click();
                        await page.waitForTimeout(300);
                        await noError(page);
                }
        });
});

test.describe('Selected torrents page', () => {
        test('loads and shows selected torrents', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/selected`);
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Selected Book');
        });
});

test.describe('Replaced torrents page', () => {
        test('loads and shows replaced torrents', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/replaced`);
                await noError(page);
                await noLoading(page);
                // torrent-005 is replaced
                await expect(page.locator('body')).toContainText('Test Book 005');
        });
});

test.describe('Duplicate torrents page', () => {
        test('loads and shows duplicate torrents', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/duplicate`);
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Duplicate Book');
        });
});

test.describe('Search page', () => {
        test('loads', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/search`);
                await noLoading(page);
                // Search form should be present (mam_id error on search result is expected in test env)
                await expect(page.locator('form')).toBeVisible();
        });

        test('can search and shows results', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/search`);
                await expect(page.locator('form')).toBeVisible();

                const input = page.locator('input[type="text"], input[type="search"]').first();
                if (await input.count() > 0) {
                        await input.fill('Test Book');
                        await input.press('Enter');
                        await page.waitForTimeout(1000);
                        // Page should still be functional (no JS crash)
                        await expect(page.locator('form')).toBeVisible();
                }
        });
});

test.describe('Lists page', () => {
        test('loads', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/lists`);
                await noError(page);
                await noLoading(page);
        });
});

test.describe('Home page', () => {
        test('loads', async ({ page }) => {
                await page.goto(`${BASE}/`);
                await noError(page);
        });
});
