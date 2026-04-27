import { expect, test, type Page } from "@playwright/test";
import { dismissActivateAccountModal, registerUser } from "../utils/registerUser";
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
      const sendTab = page.getByRole("button", { name: "Send" }).first();
      await expect(sendTab).toBeVisible();

      // Spot-Perp tab should NOT be visible
      const spotPerpTab = page.getByRole("button", { name: /spot.*perp/i });
      await expect(spotPerpTab).not.toBeVisible();
    });

    test("send button is disabled", async ({ page }) => {
      // The submit button says "Send" and should be disabled
      const sendButton = page.locator("form").getByRole("button", { name: "Send" });
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
      if (sharedPage) {
        await sharedPage.close();
      }
    });

    test("both send and spot-perp tabs are visible", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const sendTab = sharedPage.getByRole("button", { name: "Send" }).first();
      await expect(sendTab).toBeVisible();

      const spotPerpTab = sharedPage.getByRole("button", { name: /spot.*perp/i });
      await expect(spotPerpTab).toBeVisible();
    });

    test("send tab is default selected", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      // Send form should be visible
      const sendButton = sharedPage.locator("form").getByRole("button", { name: "Send" });
      await expect(sendButton).toBeVisible();

      const amountInput = sharedPage.getByRole("textbox").first();
      await expect(amountInput).toBeVisible();
    });

    test("clicking spot-perp tab shows transfer form", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: /spot.*perp/i });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      // From and To labels should be visible
      await expect(sharedPage.getByText("From")).toBeVisible();
      await expect(sharedPage.getByText("To").first()).toBeVisible();

      // Flip direction button (wrapping IconTwoArrows svg) should be visible
      const flipButton = sharedPage.getByTestId("flip-direction");
      await expect(flipButton).toBeVisible();
    });

    test("spot-perp tab shows direction labels", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: /spot.*perp/i });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      // Default direction is spot-to-perp
      await expect(sharedPage.locator('input[name="from"]')).toHaveValue("Spot Account");
      await expect(sharedPage.locator('input[name="to"]')).toHaveValue("Perp Account");
    });

    test("spot-perp direction can be flipped", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: /spot.*perp/i });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      // Get the From/To input values before flip
      const fromInput = sharedPage.locator("input[readonly]").first();
      const initialFromValue = await fromInput.inputValue();

      // Click the flip direction button
      const flipButton = sharedPage.getByTestId("flip-direction");
      await flipButton.click();
      await sharedPage.waitForTimeout(300);

      // After flip, the from value should have changed
      const newFromValue = await fromInput.inputValue();
      expect(newFromValue).not.toBe(initialFromValue);
    });

    test("spot-perp tab shows amount input and receive preview", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);
      await dismissActivateAccountModal(sharedPage);

      const spotPerpTab = sharedPage.getByRole("button", { name: /spot.*perp/i });
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(500);

      // Amount input should be visible
      const amountInput = sharedPage.getByRole("textbox").first();
      await expect(amountInput).toBeVisible();

      // "You receive" label should be visible
      await expect(sharedPage.getByText("You receive")).toBeVisible();
    });

    test("can switch between send and spot-perp tabs", async () => {
      await sharedPage.goto("/transfer");
      await waitForStorageHydration(sharedPage);

      const sendTab = sharedPage.getByRole("button", { name: "Send" }).first();
      const spotPerpTab = sharedPage.getByRole("button", { name: /spot.*perp/i });

      // Switch to spot-perp
      await spotPerpTab.click();
      await sharedPage.waitForTimeout(300);

      await expect(sharedPage.getByText("From")).toBeVisible();

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
