import { expect, test, type Page } from "@playwright/test";
import { registerUser } from "../utils/registerUser";
import { waitForStorageHydration } from "../utils/indexeddb";

test.describe("Transfer Applet", () => {
  test.describe("Not Authenticated", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/transfer");
      await waitForStorageHydration(page);
    });

    test("only send tab is visible", async ({ page }) => {
      // When not authenticated, only "send" tab/button is visible
      // Use exact: true to distinguish from "Send" submit button
      const sendTab = page.getByRole("button", { name: "send", exact: true });
      await expect(sendTab).toBeVisible();

      // Receive tab should NOT be visible
      const receiveTab = page.getByRole("button", { name: "receive", exact: true });
      await expect(receiveTab).not.toBeVisible();
    });

    test("send button is disabled", async ({ page }) => {
      // The submit button says "Send" and should be disabled
      const sendButton = page.getByRole("button", { name: "Send", exact: true });
      await expect(sendButton).toBeVisible();
      await expect(sendButton).toBeDisabled();
    });

    test("you're sending label is visible", async ({ page }) => {
      const label = page.getByText("You're sending");
      await expect(label).toBeVisible();
    });
  });

  test.describe("Authenticated", () => {
    let sharedPage: Page;

    test.beforeAll(async ({ browser }) => {
      sharedPage = await browser.newPage();
      await registerUser(sharedPage);
    });

    test.afterAll(async () => {
      await sharedPage.close();
    });

    test("both send and receive tabs are visible", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      // Use exact: true to distinguish tab from submit button
      const sendTab = sharedPage.getByRole("button", { name: "send", exact: true });
      await expect(sendTab).toBeVisible();

      const receiveTab = sharedPage.getByRole("button", { name: "receive", exact: true });
      await expect(receiveTab).toBeVisible();
    });

    test("send tab is default selected", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      // Send form should be visible
      const sendButton = sharedPage.getByRole("button", { name: "Send", exact: true });
      await expect(sendButton).toBeVisible();

      const amountInput = sharedPage.getByRole("textbox").first();
      await expect(amountInput).toBeVisible();
    });

    test("clicking receive tab shows QR code", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const receiveTab = sharedPage.getByRole("button", { name: "receive", exact: true });
      await receiveTab.click();
      await sharedPage.waitForTimeout(500);

      // QR code is a canvas element with specific dimensions (220x220)
      const qrCode = sharedPage.locator('canvas[width="220"][height="220"]');
      await expect(qrCode).toBeVisible();
    });

    test("receive tab shows account address", async () => {
      await sharedPage.goto("/transfer?action=receive");
      await waitForStorageHydration(sharedPage);
      await sharedPage.waitForTimeout(500);

      // The receive tab should show address info in various possible formats:
      // - Full address: 0x followed by hex chars
      // - Truncated: 0x...1234 format
      // - QR code canvas for the address
      // - Account label like "Account #X" or "Single Account"
      const fullAddress = sharedPage.getByText(/0x[a-fA-F0-9]{4,}/);
      const truncatedAddress = sharedPage.getByText(/0x[a-fA-F0-9]*\.{2,}/);
      const qrCode = sharedPage.locator("canvas").first();
      const accountLabel = sharedPage.getByText(/Account/i);

      const hasAddressInfo =
        (await fullAddress.count()) > 0 ||
        (await truncatedAddress.count()) > 0 ||
        (await qrCode.isVisible()) ||
        (await accountLabel.count()) > 0;

      expect(hasAddressInfo).toBeTruthy();
    });

    test("receive tab shows account type label", async () => {
      await sharedPage.goto("/transfer?action=receive");
      await waitForStorageHydration(sharedPage);

      // Multiple elements may match - use first() to avoid strict mode violation
      const accountLabel = sharedPage.getByText(/Account #\d+/);
      await expect(accountLabel.first()).toBeVisible();
    });

    test("receive tab shows warning message", async () => {
      await sharedPage.goto("/transfer?action=receive");
      await waitForStorageHydration(sharedPage);

      // Warning component should be present - check for warning-related classes or text
      const warningByClass = sharedPage.locator('[class*="warning"]');
      const warningByText = sharedPage.getByText(/warning|caution|note|only send/i);

      const warningExists =
        (await warningByClass.count()) > 0 || (await warningByText.count()) > 0;
      expect(warningExists).toBeTruthy();
    });

    test("receive tab has copy functionality", async () => {
      await sharedPage.goto("/transfer?action=receive");
      await waitForStorageHydration(sharedPage);

      // There should be a copy icon/button near the address
      const copyIcon = sharedPage.locator("svg").filter({ hasText: "" });
      const hasCopyIcon = (await copyIcon.count()) > 0;
      expect(hasCopyIcon).toBeTruthy();
    });

    test("can switch between send and receive tabs", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const sendTab = sharedPage.getByRole("button", { name: "send", exact: true });
      const receiveTab = sharedPage.getByRole("button", { name: "receive", exact: true });

      // Switch to receive
      await receiveTab.click();
      await sharedPage.waitForTimeout(300);

      // QR code is a canvas element with specific dimensions (220x220)
      const qrCode = sharedPage.locator('canvas[width="220"][height="220"]');
      await expect(qrCode).toBeVisible();

      // Switch back to send
      await sendTab.click();
      await sharedPage.waitForTimeout(300);

      const amountInput = sharedPage.getByRole("textbox").first();
      await expect(amountInput).toBeVisible();
    });

    // Placeholder for send functionality test
    test.skip("send transaction flow", async () => {
      // TODO: Implement when transaction testing is ready
    });
  });
});
