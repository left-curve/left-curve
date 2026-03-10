import type { Page } from "@playwright/test";
import type { Hex } from "viem";
import { injectMockWallet } from "./injectWallet";

export interface RegisterUserOptions {
  privateKey?: Hex;
}

/**
 * Registers a new user using the Mock E2E Wallet.
 * This utility handles the full registration flow:
 * 1. Injects the mock wallet
 * 2. Navigates to the app
 * 3. Opens the login modal and switches to registration
 * 4. Connects with the mock wallet
 * 5. Closes the account activation modal
 *
 * After this function completes, the user is logged in but with an inactive account.
 */
export async function registerUser(page: Page, options: RegisterUserOptions = {}): Promise<void> {
  // Inject the mock wallet before navigation
  await injectMockWallet(page, options);

  // Navigate to the app
  await page.goto("/");

  // Click on login button in the header
  await page.locator("[dng-connect-button]").click();

  // Wait for the modal to appear and click "Register"
  await page.getByText("Register").click();

  // Click "Connect wallet" button
  await page.getByText("Connect wallet").click();

  // Select "Mock E2E Wallet" from the wallet list
  await page.getByText("Mock E2E Wallet").click();

  // Wait for registration to complete.
  // Depending on timing/backend state, UI may show either:
  // 1) "Log In" button (then user clicks it), or
  // 2) directly "Activate Account" modal.
  const modal = page.locator(".fixed.z-\\[60\\]");
  const loginButton = modal.getByRole("button", { name: "Log In" });
  const activateHeading = page.getByRole("heading", { name: "Activate Account" });

  const registrationOutcome = await Promise.race([
    loginButton.waitFor({ state: "visible", timeout: 30_000 }).then(() => "login"),
    activateHeading.waitFor({ state: "visible", timeout: 30_000 }).then(() => "activate"),
  ]);

  if (registrationOutcome === "login") {
    await loginButton.click();
    await activateHeading.waitFor({ state: "visible" });
  }

  // At this point, Activate Account modal should be visible.
  await activateHeading.waitFor({ state: "visible" });

  // Close the modal by clicking the close button (X icon in top right)
  // The close button is inside the modal, after the content
  const closeButton = modal.locator("button").last();
  await closeButton.click();

  // Wait for modal to close
  await activateHeading.waitFor({ state: "hidden" });
}
