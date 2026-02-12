import type { Page, Request, Response } from "@playwright/test";
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
  const networkErrors: Array<Record<string, unknown>> = [];
  const onRequestFailed = (request: Request) => {
    const failure = request.failure();
    networkErrors.push({
      type: "requestfailed",
      method: request.method(),
      url: request.url(),
      errorText: failure?.errorText,
    });
  };
  const onResponse = (response: Response) => {
    if (response.status() >= 400) {
      networkErrors.push({
        type: "http_error",
        status: response.status(),
        url: response.url(),
      });
    }
  };
  page.on("requestfailed", onRequestFailed);
  page.on("response", onResponse);

  // Capture frontend runtime errors so e2e failures include UI-side root causes.
  await page.addInitScript(() => {
    const target = window as unknown as {
      __E2E_FRONTEND_ERRORS__?: Array<Record<string, unknown>>;
    };

    if (target.__E2E_FRONTEND_ERRORS__) return;
    target.__E2E_FRONTEND_ERRORS__ = [];

    const push = (entry: Record<string, unknown>) => {
      target.__E2E_FRONTEND_ERRORS__?.push({ at: Date.now(), ...entry });
    };

    window.addEventListener("error", (event) => {
      push({
        type: "window.error",
        message: event.message,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
      });
    });

    window.addEventListener("unhandledrejection", (event) => {
      push({
        type: "window.unhandledrejection",
        reason: String(event.reason),
      });
    });

    const originalConsoleError = console.error.bind(console);
    console.error = (...args: unknown[]) => {
      push({
        type: "console.error",
        args: args.map((arg) => {
          if (typeof arg === "string") return arg;
          try {
            return JSON.stringify(arg);
          } catch {
            return String(arg);
          }
        }),
      });
      originalConsoleError(...args);
    };
  });

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
  ]).catch(async (error) => {
    const frontendErrors = await page
      .evaluate(
        () =>
          (
            window as unknown as {
              __E2E_FRONTEND_ERRORS__?: Array<Record<string, unknown>>;
            }
          ).__E2E_FRONTEND_ERRORS__ ?? [],
      )
      .catch(() => []);

    const diagnostics = await page
      .evaluate(() => ({
        url: window.location.href,
        frontendChainId: window.dango?.chain?.id,
        upUrl: window.dango?.urls?.upUrl,
        modalPresent: !!document.querySelector(".fixed.z-\\[60\\]"),
      }))
      .catch(() => null);

    const modalButtons = await modal.getByRole("button").allTextContents().catch(() => []);
    const modalText = await modal.innerText().catch(() => "<modal not readable>");
    const recentNetworkErrors = networkErrors.slice(-20);

    console.error("[e2e/registerUser] Timed out waiting for login/activate state", {
      diagnostics,
      frontendErrors,
      recentNetworkErrors,
      modalButtons,
      modalText,
      originalError: String(error),
    });

    page.off("requestfailed", onRequestFailed);
    page.off("response", onResponse);

    throw new Error(
      `[e2e/registerUser] registration flow timeout: ${String(error)}\n` +
        `diagnostics=${JSON.stringify(diagnostics)}\n` +
        `frontendErrors=${JSON.stringify(frontendErrors)}\n` +
        `recentNetworkErrors=${JSON.stringify(recentNetworkErrors)}\n` +
        `modalButtons=${JSON.stringify(modalButtons)}\n` +
        `modalText=${JSON.stringify(modalText)}`,
    );
  });

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

  page.off("requestfailed", onRequestFailed);
  page.off("response", onResponse);
}
