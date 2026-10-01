import { defineConfig, devices } from '@playwright/test'
export default defineConfig({
  testDir: './tests',
  testMatch: '**/*.spec.ts',
  workers: 1,
  use: { baseURL: 'http://127.0.0.1:43123', trace: 'retain-on-failure' },
  projects: [
    { name: 'desktop', use: { ...devices['Desktop Chrome'] } },
    { name: 'mobile', use: { ...devices['Pixel 7'] } },
    ...(process.env.SPLITSHARE_BROWSER_MATRIX ? [
      { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
      { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    ] : []),
  ],
  webServer: { command: 'node tests/start-server.mjs', url: 'http://127.0.0.1:43123', reuseExistingServer: false },
})
