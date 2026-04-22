import type { Page } from "@playwright/test";
import type { Hex } from "viem";
import { injectMockWallet } from "./injectWallet";

export interface RegisterUserOptions {
  privateKey?: Hex;
}

export async function dismissActivateAccountModal(
  page: Page,
  timeout = 2_000,
): Promise<void> {
  const heading = page.getByRole("heading", { name: "Activate Account" });

  const isVisible = await heading
    .waitFor({ state: "visible", timeout })
    .then(() => true)
    .catch(() => false);

  if (!isVisible) {
    return;
  }

  await page.getByText("do this later", { exact: false }).click();
  await heading.waitFor({ state: "hidden", timeout: 10_000 });
}

/**
 * Registers a new user using the Mock E2E Wallet.
 * This utility handles the full registration flow:
 * 1. Injects the mock wallet
 * 2. Navigates to the app
 * 3. Opens the auth modal
 * 4. Connects with the mock wallet
 * 5. Creates account or picks an existing one
 * 6. Dismisses any post-login modals
 *
 * After this function completes, the user is logged in.
 */
export async function registerUser(page: Page, options: RegisterUserOptions = {}): Promise<void> {
  // Inject the mock wallet before navigation
  await injectMockWallet(page, options);

  // Navigate to the app
  await page.goto("/");

  // Click on login button in the header
  await page.locator("[dng-connect-button]").click();

  // Click "Connect Wallet" on the welcome screen
  await page.getByText("Connect Wallet", { exact: true }).click();

  // Select "Mock E2E Wallet" from the wallet list
  await page.getByText("Mock E2E Wallet").click();

  // After wallet authentication, the flow shows either:
  // 1) "create-account" screen (new wallet) with a "Continue" button, or
  // 2) "account-picker" screen (existing wallet) with username list.
  const modal = page.locator(".fixed.z-\\[60\\]");
  const continueButton = modal.getByRole("button", { name: "Continue" });
  const usernamesHeading = modal.getByRole("heading", { name: "Usernames found" });

  const authOutcome = await Promise.race([
    continueButton.waitFor({ state: "visible", timeout: 30_000 }).then(() => "create"),
    usernamesHeading.waitFor({ state: "visible", timeout: 30_000 }).then(() => "pick"),
  ]);

  if (authOutcome === "create") {
    // New wallet — create account
    await continueButton.click();
  } else {
    // Existing wallet — select the first username (JS click to bypass viewport scroll issues)
    await modal.locator('img[alt="username"]').first().dispatchEvent("click");
  }

  // Wait for login to complete (header shows account info)
  await page.locator("[dng-connect-button]").filter({ hasText: /Account #/ }).waitFor({
    state: "visible",
    timeout: 30_000,
  });

  // Auto-dismiss ActivateAccount modal whenever it appears.
  // The modal re-triggers on every full navigation (page.goto) because the
  // React ref that guards it resets on remount, so a one-time check is not enough.
  await page.addLocatorHandler(
    page.getByRole("heading", { name: "Activate Account" }),
    async () => {
      await dismissActivateAccountModal(page, 10_000);
    },
  );
}
