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

  // Wait for the registration to complete and the "Log In" button to appear in the modal
  // Use the modal container to scope the selector (avoid matching the header button)
  const modal = page.locator(".fixed.z-\\[60\\]");
  const loginButton = modal.getByRole("button", { name: "Log In" });

  await loginButton.waitFor({ state: "visible" });

  await loginButton.click();

  // Wait for the "Activate Account" modal to appear
  await page.getByRole("heading", { name: "Activate Account" }).waitFor({ state: "visible" });

  // Close the modal by clicking the close button (X icon in top right)
  // The close button is inside the modal, after the content
  const closeButton = modal.locator("button").last();
  await closeButton.click();

  // Wait for modal to close
  await page.getByRole("heading", { name: "Activate Account" }).waitFor({ state: "hidden" });
}
