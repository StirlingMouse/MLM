import { defineConfig } from '@playwright/test';

export default defineConfig({
        testDir: './tests/e2e',
        globalSetup: './tests/e2e/setup.ts',
        globalTeardown: './tests/e2e/teardown.ts',
        timeout: 30_000,
        use: {
                baseURL: 'http://localhost:3998',
                headless: true,
        },
        projects: [
                { name: 'chromium', use: { browserName: 'chromium' } },
        ],
        // Allow initial server startup time
        expect: { timeout: 15_000 },
});
