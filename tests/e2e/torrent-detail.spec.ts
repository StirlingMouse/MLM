import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:3998';
// torrent-001 is a known test torrent with a library path
const DETAIL_URL = `${BASE}/dioxus/torrents/torrent-001`;

test.describe('Torrent detail page', () => {
        test('client fetches and renders qBittorrent data', async ({ page }) => {
                const qbitRequest = page.waitForRequest(
                        req => req.method() === 'POST' && req.url().includes('/api/get_qbit_data'),
                        { timeout: 20_000 }
                );
                const qbitResponse = page.waitForResponse(
                        res =>
                                res.request().method() === 'POST' &&
                                res.url().includes('/api/get_qbit_data') &&
                                res.status() === 200,
                        { timeout: 20_000 }
                );
                await page.goto(DETAIL_URL);

                await qbitRequest;
                await qbitResponse;

                await expect(page.locator('h3', { hasText: 'qBittorrent' })).toBeVisible({
                        timeout: 20_000,
                });
                await expect(page.locator('dd', { hasText: 'Stalled (Seeding)' })).toBeVisible({
                        timeout: 20_000,
                });
        });

        test('client fetches and renders other torrents data', async ({ page }) => {
                const otherRequest = page.waitForRequest(
                        req =>
                                req.method() === 'POST' &&
                                req.url().includes('/api/get_other_torrents'),
                        { timeout: 20_000 }
                );
                const otherResponse = page.waitForResponse(
                        res =>
                                res.request().method() === 'POST' &&
                                res.url().includes('/api/get_other_torrents') &&
                                res.status() === 200,
                        { timeout: 20_000 }
                );

                await page.goto(DETAIL_URL);

                await otherRequest;
                await otherResponse;

                await expect(page.locator('h3', { hasText: 'Other Torrents' })).toBeVisible({
                        timeout: 20_000,
                });
                await expect(page.locator('body')).toContainText('Mock Search: Way of Kings', {
                        timeout: 20_000,
                });
        });

        test('loads and shows torrent info', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await expect(page.locator('.error')).toHaveCount(0);
                // Should show the torrent title
                await expect(page.locator('body')).toContainText('Test Book 001');
        });

        test('other torrents section resolves (not stuck loading)', async ({ page }) => {
                await page.goto(DETAIL_URL);

                // Wait for "Other Torrents" heading to appear
                await expect(page.locator('h3', { hasText: 'Other Torrents' })).toBeVisible({
                        timeout: 20_000,
                });

                // The loading indicator should disappear as client fetches data
                await expect(
                        page.locator('.loading-indicator', { hasText: 'Loading other torrents...' })
                ).toHaveCount(0, { timeout: 20_000 });
        });

        test('qbit section is not stuck loading', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await page.waitForTimeout(5_000); // give client-side fetches time to complete

                // qbit loading indicator must be gone (either data or nothing rendered)
                await expect(
                        page.locator('.loading-indicator', { hasText: 'Loading qBittorrent data...' })
                ).toHaveCount(0);
        });

        test('no error state on initial load', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await expect(page.locator('.error')).toHaveCount(0);
        });

        test('replaced torrent detail loads', async ({ page }) => {
                // torrent-005 is replaced by torrent-006 in our test data
                await page.goto(`${BASE}/dioxus/torrents/torrent-005`);
                await expect(page.locator('.error')).toHaveCount(0);
                await expect(page.locator('body')).toContainText('Test Book 005');
        });
});
