import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("Landing render", async ({ page }) => {
  await expect(page.locator("text=Learn More")).toBeVisible();
});
