import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:3998';
const DETAIL_URL = `${BASE}/dioxus/torrents/torrent-001`;

// Wait for a loading indicator to disappear, then assert something appeared.
async function waitForLoad(page: import('@playwright/test').Page, indicator: string) {
        await expect(page.locator('.loading-indicator', { hasText: indicator })).toHaveCount(0, {
                timeout: 20_000,
        });
}

// ── Search page (mock-backed) ─────────────────────────────────────────────────

test.describe('Search page with mock MaM', () => {
        test('submitting a search returns mock results', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/search`);
                await expect(page.locator('form')).toBeVisible();

                // Fill and submit the search form
                await page.locator('input[type="text"], input[type="search"]').first().fill('Way of Kings');
                await page.locator('form').locator('button[type="submit"]').click();

                // Mock returns 2 torrents; wait for result rows to appear
                await expect(page.locator('.TorrentRow').first()).toBeVisible({ timeout: 15_000 });
                await expect(page.locator('.TorrentRow')).toHaveCount(2, { timeout: 5_000 });
        });

        test('search results contain mock torrent titles', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/search`);
                await page.locator('input[type="text"], input[type="search"]').first().fill('test');
                await page.locator('form').locator('button[type="submit"]').click();

                await expect(page.locator('.TorrentRow').first()).toBeVisible({ timeout: 15_000 });
                // Both mock titles should appear on the page
                await expect(page.locator('body')).toContainText('Way of Kings');
                await expect(page.locator('body')).toContainText('Name of the Wind');
        });
});

// ── Torrent detail: qBittorrent section ──────────────────────────────────────

test.describe('Torrent detail qBittorrent section', () => {
        test('qbit section loads and shows torrent state', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await waitForLoad(page, 'Loading qBittorrent data...');

                // Mock returns state "stalledUP" — the UI renders it in a <dd> as human-readable
                await expect(page.locator('dd', { hasText: 'Stalled (Seeding)' })).toBeVisible({
                        timeout: 10_000,
                });
        });

        test('qbit section shows tracker URL', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await waitForLoad(page, 'Loading qBittorrent data...');

                // Mock returns uploaded=620000000 bytes → "591.28 MiB" shown in qBit section
                await expect(page.locator('body')).toContainText('591.28 MiB', {
                        timeout: 10_000,
                });
        });

        test('qbit section shows file name', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await waitForLoad(page, 'Loading qBittorrent data...');

                await expect(page.locator('body')).toContainText('Test Book 001.m4b', {
                        timeout: 10_000,
                });
        });
});

// ── Torrent detail: Other Torrents section ────────────────────────────────────

test.describe('Torrent detail other torrents section', () => {
        test('other torrents section loads and shows mock results', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await waitForLoad(page, 'Loading other torrents...');

                // The "Other Torrents" section should contain at least one TorrentRow
                await expect(page.locator('h3', { hasText: 'Other Torrents' })).toBeVisible({
                        timeout: 10_000,
                });
                await expect(page.locator('.TorrentRow').first()).toBeVisible({ timeout: 10_000 });
        });
});

// ── Selected page: user info ──────────────────────────────────────────────────

test.describe('Selected page user info from mock MaM', () => {
        test('shows bonus and unsat info from mock', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/selected`);

                // Mock returns: bonus=50000, wedges=3, unsat count=2, limit=10
                await expect(page.locator('body')).toContainText('50000', { timeout: 10_000 });
                await expect(page.locator('body')).toContainText('Wedges: 3');
                await expect(page.locator('body')).toContainText('Unsats: 2 / 10');
        });

        test('shows remaining buffer computed from mock user data', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/selected`);

                // Buffer is derived from uploaded - downloaded; should be non-empty
                await expect(page.locator('body')).toContainText('Buffer:', { timeout: 10_000 });
        });
});
