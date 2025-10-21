import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("Check up endpoint", async ({ page }) => {
  const response = await page.waitForResponse(new RegExp(`/up`));
  expect(response.status()).toBe(200);
});
