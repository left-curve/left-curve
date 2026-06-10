import { expect, test, type Page } from "@playwright/test";

import { dismissActivateAccountModal, registerUser } from "../utils/registerUser";
import { waitForStorageHydration } from "../utils/indexeddb";
import { message } from "../utils/messages";

const transferLabels = {
  from: message("transfer.spotPerp.from"),
  perpAccount: message("accountMenu.perpAccount"),
  send: message("common.send"),
  spotAccount: message("accountMenu.spotAccount"),
  spotPerp: message("accountMenu.spotPerp"),
  to: message("transfer.spotPerp.to"),
  youReceive: message("transfer.spotPerp.youReceive"),
};

test.describe("Transfer Applet", () => {
  test.describe("Not Authenticated", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/transfer");
      await waitForStorageHydration(page);
    });

    test("only send tab is visible", async ({ page }) => {
      const sendTab = page.getByRole("button", { name: transferLabels.send }).first();
      await expect(sendTab).toBeVisible();

      const spotPerpTab = page.getByRole("button", { name: transferLabels.spotPerp });
      await expect(spotPerpTab).not.toBeVisible();
    });

    test("send button is disabled", async ({ page }) => {
      const sendButton = page.locator("form").getByRole("button", { name: transferLabels.send });
      await expect(sendButton).toBeVisible();
      await expect(sendButton).toBeDisabled();
    });

    test("amount input is visible", async ({ page }) => {
      await expect(page.locator('input[name="amount"]')).toBeVisible();
    });
  });

  test.describe("Authenticated", () => {
    let sharedPage: Page;

    test.beforeAll(async ({ browser }) => {
      sharedPage = await browser.newPage();
      await registerUser(sharedPage);
    });

    test.afterAll(async () => {
      if (sharedPage) {
        await sharedPage.close();
      }
    });

    test("both send and spot-perp tabs are visible", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const sendTab = sharedPage.getByRole("button", { name: transferLabels.send }).first();
      await expect(sendTab).toBeVisible();

      const spotPerpTab = sharedPage.getByRole("button", { name: transferLabels.spotPerp });
      await expect(spotPerpTab).toBeVisible();
    });

    test("send tab is default selected", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const sendButton = sharedPage
        .locator("form")
        .getByRole("button", { name: transferLabels.send });
      await expect(sendButton).toBeVisible();

      const amountInput = sharedPage.getByRole("textbox").first();
      await expect(amountInput).toBeVisible();
    });

    test("clicking spot-perp tab shows transfer form", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: transferLabels.spotPerp });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      await expect(sharedPage.locator('input[name="from"]')).toBeVisible();
      await expect(sharedPage.locator('input[name="to"]')).toBeVisible();

      const flipButton = sharedPage.getByTestId("flip-direction");
      await expect(flipButton).toBeVisible();
    });

    test("spot-perp tab shows direction labels", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: transferLabels.spotPerp });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      await expect(sharedPage.locator('input[name="from"]')).toHaveValue(
        transferLabels.spotAccount,
      );
      await expect(sharedPage.locator('input[name="to"]')).toHaveValue(
        transferLabels.perpAccount,
      );
    });

    test("spot-perp direction can be flipped", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: transferLabels.spotPerp });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      const fromInput = sharedPage.locator("input[readonly]").first();
      const initialFromValue = await fromInput.inputValue();

      const flipButton = sharedPage.getByTestId("flip-direction");
      await flipButton.click();
      await sharedPage.waitForTimeout(300);

      const newFromValue = await fromInput.inputValue();
      expect(newFromValue).not.toBe(initialFromValue);
    });

    test("spot-perp tab shows amount input and receive preview", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);
      await dismissActivateAccountModal(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: transferLabels.spotPerp });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      const amountInput = sharedPage.getByRole("textbox").first();
      await expect(amountInput).toBeVisible();

      await expect(sharedPage.getByText(transferLabels.youReceive)).toBeVisible();
    });

    test("can switch between send and spot-perp tabs", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const sendTab = sharedPage.getByRole("button", { name: transferLabels.send }).first();
      const spotPerpTab = sharedPage.getByRole("button", { name: transferLabels.spotPerp });

      await spotPerpTab.click();
      await sharedPage.waitForTimeout(300);

      await expect(sharedPage.locator('input[name="from"]')).toBeVisible();

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
