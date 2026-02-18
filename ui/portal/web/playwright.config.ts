import { defineConfig, devices } from "@playwright/test";

const frontendUrl = process.env.FRONTEND_URL || "http://127.0.0.1:5080";
const useExternalServer = process.env.PLAYWRIGHT_EXTERNAL_SERVER === "1";

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  outputDir: "./tests/reports",
  testDir: "./tests",
  /* Run tests in files in parallel */
  fullyParallel: false,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  retries: 0,
  webServer: useExternalServer
    ? undefined
    : {
        command: "npm run dev",
        url: frontendUrl,
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
      },
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: frontendUrl,

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: "on-first-retry",
    bypassCSP: true,
    launchOptions: {
      args: ["--disable-web-security"],
    },
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
