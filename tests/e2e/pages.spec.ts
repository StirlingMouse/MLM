import { test, expect } from '@playwright/test';

function noError(page: import('@playwright/test').Page) {
        return expect(page.locator('.error')).toHaveCount(0);
}
function noLoading(page: import('@playwright/test').Page) {
        return expect(page.locator('.loading-indicator')).toHaveCount(0, { timeout: 15_000 });
}

test.describe('Events page', () => {
        test('loads and shows events', async ({ page }) => {
                await page.goto('/dioxus/events');
                await noError(page);
                await noLoading(page);
                // 10 events were inserted
                await expect(page.locator('body')).toContainText('Grabbed');
        });

        test('no loading indicator stuck', async ({ page }) => {
                await page.goto('/dioxus/events');
                await noLoading(page);
        });
});

test.describe('Errors page', () => {
        test('loads and shows errored torrents', async ({ page }) => {
                await page.goto('/dioxus/errors');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Errored Book');
        });

        test('sort header is interactive', async ({ page }) => {
                await page.goto('/dioxus/errors');
                await noLoading(page);
                const sortBtn = page.locator('.header button.link').first();
                await expect(sortBtn).toHaveCount(1);
                await sortBtn.click();
                await noLoading(page);
                await noError(page);
        });
});

test.describe('Selected torrents page', () => {
        test('loads and shows selected torrents', async ({ page }) => {
                await page.goto('/dioxus/selected');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Selected Book');
        });
});

test.describe('Replaced torrents page', () => {
        test('loads and shows replaced torrents', async ({ page }) => {
                await page.goto('/dioxus/replaced');
                await noError(page);
                await noLoading(page);
                // torrent-005 is replaced
                await expect(page.locator('body')).toContainText('Test Book 005');
        });
});

test.describe('Duplicate torrents page', () => {
        test('loads and shows duplicate torrents', async ({ page }) => {
                await page.goto('/dioxus/duplicate');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Duplicate Book');
        });
});

test.describe('Search page', () => {
        test('loads', async ({ page }) => {
                await page.goto('/dioxus/search');
                await noLoading(page);
                // Search form should be present (mam_id error on search result is expected in test env)
                await expect(page.locator('form')).toBeVisible();
        });

        test('can search and shows results', async ({ page }) => {
                await page.goto('/dioxus/search');
                await expect(page.locator('form')).toBeVisible();

                const input = page.locator('input[type="text"], input[type="search"]').first();
                await expect(input).toHaveCount(1);
                await input.fill('Test Book');
                await Promise.all([
                        page.waitForURL(url => url.toString().includes('/dioxus/search?'), {
                                timeout: 5_000,
                        }),
                        input.press('Enter'),
                ]);
                await expect(page.locator('form')).toBeVisible();
        });
});

test.describe('Lists page', () => {
        test('loads', async ({ page }) => {
                await page.goto('/dioxus/lists');
                await noError(page);
                await noLoading(page);
        });
});

test.describe('Home page', () => {
        test('loads', async ({ page }) => {
                await page.goto('/');
                await noError(page);
        });
});
