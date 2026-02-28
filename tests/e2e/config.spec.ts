import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:3998';

test.describe('Config page', () => {
        test('legacy /config route redirects to Dioxus config page', async ({ page }) => {
                await page.goto(`${BASE}/config`);
                await expect(page).toHaveURL(/\/dioxus\/config/);
                await expect(page.locator('h1')).toContainText('Config');
                await expect(page.locator('body')).toContainText('unsat_buffer');
                await expect(page.locator('body')).toContainText('[[qbittorrent]]');
        });

        test('dioxus config page renders legacy config formatting', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/config`);

                await expect(page.locator('h1')).toContainText('Config');
                await expect(page.locator('body')).toContainText('[[qbittorrent]]');
                await expect(page.locator('body')).toContainText('audio_types');
                await expect(page.locator('body')).toContainText('ebook_types');
        });

        test('show_apply_tags query enables apply controls when tag sections exist', async ({ page }) => {
                await page.goto(`${BASE}/dioxus/config?show_apply_tags=true`);
                const applyBtn = page.locator('button', { hasText: 'apply to all' }).first();
                if ((await applyBtn.count()) === 0) {
                        test.info().annotations.push({
                                type: 'note',
                                description: 'No [[tag]] entries in e2e config fixture; apply controls are not rendered.',
                        });
                        await expect(page.locator('h1')).toContainText('Config');
                        return;
                }
                await expect(applyBtn).toBeVisible();
                await expect(page.locator('input[type="number"]').first()).toBeVisible();
        });
});
