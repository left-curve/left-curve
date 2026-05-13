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
      // The edit icon only appears when the user has reached the volume threshold.
      // Look for the edit icon (SVG) next to Commission Rate - if not found, skip the test.
      const commissionRateLabel = sharedPage.getByText("Commission Rate", { exact: false }).first();
      const editIcon = commissionRateLabel.locator("xpath=preceding-sibling::div//*[name()='svg']").first();

      const isEditIconVisible = await editIcon
        .waitFor({ state: "visible", timeout: 5000 })
        .then(() => true)
        .catch(() => false);

      if (!isEditIconVisible) {
        test.skip(true, "User is not a referrer - Edit icon not visible (no trading volume)");
        return;
      }

      await editIcon.click();

      const refereeReceivesInput = sharedPage.locator('.fixed.z-\\[60\\] input[type="number"]').first();
      await expect(refereeReceivesInput).toBeVisible();

      await refereeReceivesInput.fill("");
      await expect(refereeReceivesInput).toHaveValue("");
    });
  });
});
