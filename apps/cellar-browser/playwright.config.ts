import { defineConfig } from '@playwright/test';
export default defineConfig({ testDir: './tests/browser', workers: 1, use: { baseURL: process.env.CELLAR_TEST_PREVIEW_URL ?? 'http://127.0.0.1:4317', headless: true, viewport: { width: 1440, height: 950 }, channel: 'chrome' }, reporter: 'list' });
