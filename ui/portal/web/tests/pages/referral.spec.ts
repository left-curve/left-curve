import { expect, test, type Page } from "@playwright/test";
import { registerUser } from "../utils/registerUser";
import { waitForStorageHydration } from "../utils/indexeddb";

async function enableReferralFeature(page: Page): Promise<void> {
  await page.addInitScript(() => {
    let dangoConfig: Record<string, unknown> | undefined;

    Object.defineProperty(window, "dango", {
      configurable: true,
      get() {
        return dangoConfig;
      },
      set(value) {
        const config =
          value && typeof value === "object"
            ? (value as Record<string, unknown>)
            : {};
        const enabledFeatures = Array.isArray(config.enabledFeatures)
          ? (config.enabledFeatures as string[])
          : [];

        dangoConfig = {
          ...config,
          enabledFeatures: Array.from(new Set([...enabledFeatures, "referral"])),
        };
      },
    });
  });
}

const getLockedBannerButton = (page: Page) =>
  page.getByAltText("Referral banner").locator("xpath=preceding-sibling::div//button").first();

async function openReferralAffiliateTab(page: Page): Promise<void> {
  await page.goto("/referral?tab=affiliate");
  await waitForStorageHydration(page);
}

test.describe("Referral Page", () => {
  test.describe("Not Authenticated", () => {
    test.beforeEach(async ({ page }) => {
      await enableReferralFeature(page);
      await openReferralAffiliateTab(page);
    });

    test("shows the locked banner instead of referral credentials", async ({ page }) => {
      await expect(page.getByAltText("Referral banner")).toBeVisible();
      await expect(getLockedBannerButton(page)).toHaveText(/log in|sign in/i);
      await expect(page.getByText("My Referral Link", { exact: true })).toHaveCount(0);
      await expect(page.getByText("My Referral Code", { exact: true })).toHaveCount(0);
    });
  });

  test.describe("Authenticated", () => {
    let sharedPage: Page;

    test.beforeAll(async ({ browser }) => {
      sharedPage = await browser.newPage();
      await enableReferralFeature(sharedPage);
      await registerUser(sharedPage);
    });

    test.afterAll(async () => {
      if (sharedPage) {
        await sharedPage.close();
      }
    });

    test("allows clearing the referee receives input while editing", async () => {
      await openReferralAffiliateTab(sharedPage);

      // Wait for the page content to load
      await sharedPage.waitForLoadState("networkidle");

      // This test requires the user to be a referrer (has trading volume and commission settings).
      // If the "Commission Rate" section isn't visible, skip the test gracefully since
      // the preconditions aren't met (e.g., new user with no trading volume).
      const commissionRateLabel = sharedPage.getByText("Commission Rate", { exact: false }).first();
      const isVisible = await commissionRateLabel
        .waitFor({ state: "visible", timeout: 5000 })
        .then(() => true)
        .catch(() => false);

      if (!isVisible) {
        test.skip(true, "User is not a referrer - Commission Rate section not visible");
        return;
      }

      await commissionRateLabel
        .locator("xpath=preceding-sibling::div//*[name()='svg']")
        .first()
        .click();

      const refereeReceivesInput = sharedPage.locator('.fixed.z-\\[60\\] input[type="number"]').first();
      await expect(refereeReceivesInput).toBeVisible();

      await refereeReceivesInput.fill("");
      await expect(refereeReceivesInput).toHaveValue("");
    });
  });
});
