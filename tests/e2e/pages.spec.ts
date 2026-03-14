import { test, expect } from '@playwright/test';

function noError(page: import('@playwright/test').Page) {
        return expect(page.locator('.error')).toHaveCount(0);
}
function noLoading(page: import('@playwright/test').Page) {
        return expect(page.locator('.loading-indicator')).toHaveCount(0, { timeout: 15_000 });
}

test.describe('Events page', () => {
        test('loads and shows events', async ({ page }) => {
                await page.goto('/events');
                await noError(page);
                await noLoading(page);
                // 10 events were inserted
                await expect(page.locator('body')).toContainText('Grabbed');
        });

        test('no loading indicator stuck', async ({ page }) => {
                await page.goto('/events');
                await noLoading(page);
        });
});

test.describe('Errors page', () => {
        test('loads and shows errored torrents', async ({ page }) => {
                await page.goto('/errors');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Errored Book');
        });

        test('sort header is interactive', async ({ page }) => {
                await page.goto('/errors');
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
                await page.goto('/selected');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Selected Book');
        });

        test('shows the selected torrent stats above the table', async ({ page }) => {
                await page.goto('/selected');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Buffer:');
                await expect(page.locator('body')).toContainText('Unsats: 2 / 10');
                await expect(page.locator('body')).toContainText('Wedges: 3');
                await expect(page.locator('body')).toContainText('Bonus: 50000');
                await expect(page.locator('body')).toContainText('Queued Torrents: 5');
                await expect(page.locator('body')).toContainText('Downloading Torrents: 0');
        });
});

test.describe('Replaced torrents page', () => {
        test('loads and shows replaced torrents', async ({ page }) => {
                await page.goto('/replaced');
                await noError(page);
                await noLoading(page);
                // torrent-005 is replaced
                await expect(page.locator('body')).toContainText('Test Book 005');
        });
});

test.describe('Duplicate torrents page', () => {
        test('loads and shows duplicate torrents', async ({ page }) => {
                await page.goto('/duplicate');
                await noError(page);
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Duplicate Book');
        });
});

test.describe('Search page', () => {
        test('loads', async ({ page }) => {
                await page.goto('/search');
                await noLoading(page);
                // Search form should be present (mam_id error on search result is expected in test env)
                await expect(page.locator('form')).toBeVisible();
        });

        test('can search and shows results', async ({ page }) => {
                await page.goto('/search');
                await expect(page.locator('form')).toBeVisible();

                const input = page.locator('input[type="text"], input[type="search"]').first();
                const submit = page.getByRole('button', { name: 'Search' });
                await expect(input).toHaveCount(1);
                await input.fill('Test Book');
                await Promise.all([
                        page.waitForURL(url => url.toString().includes('/search?'), {
                                timeout: 5_000,
                        }),
                        submit.click(),
                ]);
                await expect(page.locator('form')).toBeVisible();
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Found 205 torrents');
                await expect(page.locator('body')).toContainText('Mock Search Result 001');
                await expect(page.locator('body')).not.toContainText('Mock Search Result 101');
        });

        test('server renders paged search results into html', async ({ request }) => {
                const response = await request.get('/search?q=Test%20Book&page=2');
                expect(response.ok()).toBeTruthy();

                const html = await response.text();
                expect(html).toContain('Found 205 torrents');
                expect(html).toContain('Mock Search Result 101');
                expect(html).not.toContain('Loading search results...');
        });

        test('pagination click updates the visible result page', async ({ page }) => {
                await page.goto('/search?q=Test%20Book');
                await noLoading(page);
                await expect(page.locator('body')).toContainText('Mock Search Result 001');
                await expect(page.locator('body')).not.toContainText('Mock Search Result 101');

                await Promise.all([
                        page.waitForURL(url => url.toString().includes('page=2'), { timeout: 5_000 }),
                        page.getByRole('button', { name: '2' }).first().click(),
                ]);

                await noLoading(page);
                await expect(page.locator('body')).toContainText('Mock Search Result 101');
                await expect(page.locator('body')).not.toContainText('Mock Search Result 001');
        });
});

test.describe('Lists page', () => {
        test('loads', async ({ page }) => {
                await page.goto('/lists');
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
