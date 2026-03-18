import { test, expect } from '@playwright/test';

const DETAIL_URL = '/torrents/torrent-001';

function browserOffset(projectName: string): number {
        switch (projectName) {
                case 'chromium':
                        return 0;
                case 'firefox':
                        return 1;
                case 'webkit':
                        return 2;
                default:
                        return 0;
        }
}

function torrentIdFor(index: number): string {
        return `torrent-${String(index).padStart(3, '0')}`;
}

test.describe('Torrent detail page', () => {
        test('server renders the edit page route', async ({ request }) => {
                const response = await request.get('/torrent-edit/torrent-001');
                expect(response.ok()).toBeTruthy();

                const html = await response.text();
                expect(html).toContain('Edit Torrent Metadata');
                expect(html).not.toContain('Page Not Found');
        });

        test('edit metadata link loads the edit page', async ({ page }) => {
                await page.goto(DETAIL_URL);

                await Promise.all([
                        page.waitForURL('/torrent-edit/torrent-001', { timeout: 10_000 }),
                        page.getByRole('link', { name: 'Edit Metadata' }).click(),
                ]);

                await expect(
                        page.getByRole('heading', { name: 'Edit Torrent Metadata' })
                ).toBeVisible();
                await expect(page.locator('input[type="text"]').first()).toBeVisible();
        });

        test('can edit torrent metadata and persist the change', async ({ page }, testInfo) => {
                const torrentIndex = 11 + browserOffset(testInfo.project.name);
                const torrentId = torrentIdFor(torrentIndex);
                const updatedDescription =
                        `Description updated by Playwright for ${testInfo.project.name}.`;

                await page.goto(`/torrent-edit/${torrentId}`);
                await expect(
                        page.getByRole('heading', { name: 'Edit Torrent Metadata' })
                ).toBeVisible();

                const description = page.getByLabel('Description');
                await description.fill(updatedDescription);

                await page.getByRole('button', { name: 'Save' }).click();
                await expect(page.locator('body')).toContainText('Metadata updated');

                await page.reload();
                await expect(
                        page.getByRole('heading', { name: 'Edit Torrent Metadata' })
                ).toBeVisible();
                await expect(page.getByLabel('Description')).toHaveValue(updatedDescription);

                await page.goto(`/torrents/${torrentId}`);
                await expect(page.locator('.torrent-description')).toContainText(updatedDescription);
        });

        test('can edit identifiers and chip-based metadata fields', async ({ page }, testInfo) => {
                const torrentIndex = 14 + browserOffset(testInfo.project.name);
                const torrentId = torrentIdFor(torrentIndex);
                const updatedGoodreadsId = '7654321';
                const addedCategory = 'Cozy Mystery';

                await page.goto(`/torrent-edit/${torrentId}`);
                await expect(
                        page.getByRole('heading', { name: 'Edit Torrent Metadata' })
                ).toBeVisible();

                const categoriesEditor = page.locator('.multi-value-editor', {
                        has: page.getByRole('heading', { name: 'Categories' }),
                });

                await page.getByLabel('Goodreads ID').fill(updatedGoodreadsId);

                await categoriesEditor.getByLabel('Add category').fill('cozy');
                await categoriesEditor
                        .locator('.editor-suggestions')
                        .getByRole('button', { name: addedCategory })
                        .click();
                await expect(categoriesEditor.locator('.editor-selected')).toContainText(
                        addedCategory
                );

                await page.getByRole('button', { name: 'Save' }).click();
                await expect(page.locator('body')).toContainText('Metadata updated');

                await page.reload();
                await expect(page.getByLabel('Goodreads ID')).toHaveValue(updatedGoodreadsId);
                await expect(categoriesEditor.locator('.editor-selected')).toContainText(
                        addedCategory
                );

                await page.goto(`/torrents/${torrentId}`);
                await expect(page.getByRole('link', { name: 'Open in Goodreads' })).toHaveAttribute(
                        'href',
                        /7654321/
                );
        });

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
                await expect(page.locator('.detail-related-card')).toContainText(
                        'Mock Search Result 001',
                        {
                                timeout: 20_000,
                        }
                );
        });

        test('loads and shows torrent info', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await expect(page.locator('.error')).toHaveCount(0);
                // Should show the torrent title
                await expect(page.locator('body')).toContainText('Test Book 001');
        });

        test('uses grouped actions and shared section toggles', async ({ page }) => {
                await page.goto(DETAIL_URL);

                const descriptionSection = page
                        .locator('details')
                        .filter({ has: page.locator('summary', { hasText: 'Description' }) })
                        .first();
                await expect(descriptionSection).toHaveJSProperty('open', true);
                await expect(descriptionSection).toContainText('Description for Test Book 001');

                const historySection = page
                        .locator('details')
                        .filter({ has: page.locator('summary', { hasText: 'Event History' }) })
                        .first();
                await expect(historySection).toHaveJSProperty('open', false);
                await historySection.locator('summary').click();
                await expect(historySection).toHaveJSProperty('open', true);

                const sidebarMetadata = page.locator('.torrent-side .detail-metadata-table');
                await expect(sidebarMetadata).not.toContainText('Library Path');

                const libraryCard = page.locator('.torrent-main .detail-library-card');
                await expect(libraryCard).toContainText('Library');
                await expect(libraryCard).toContainText('/library/books/Test Book 001');
                await expect(libraryCard).toContainText('Relink');
                await expect(libraryCard).toContainText('Refresh & Relink');
                await expect(libraryCard).toContainText('Clean');
                await expect(libraryCard).toContainText('Remove');

                const metadataActions = page.locator('.detail-action-group').filter({
                        hasText: 'Metadata',
                });
                await expect(metadataActions).toContainText('Edit Metadata');
                await expect(metadataActions).toContainText('Match Metadata');
                await expect(metadataActions).toContainText('Refresh from MaM');
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
                await expect(
                        page.locator('.loading-indicator', { hasText: 'Loading qBittorrent data...' })
                ).toHaveCount(0, { timeout: 20_000 });
        });

        test('no error state on initial load', async ({ page }) => {
                await page.goto(DETAIL_URL);
                await expect(page.locator('.error')).toHaveCount(0);
        });

        test('replaced torrent detail loads', async ({ page }) => {
                // torrent-005 is replaced by torrent-006 in our test data
                await page.goto('/torrents/torrent-005');
                await expect(page.locator('.error')).toHaveCount(0);
                await expect(page.locator('body')).toContainText('Test Book 005');
        });
});
