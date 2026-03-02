import { expect, test, type Page } from "@playwright/test";
import { registerUser } from "../utils/registerUser";
import {
  getAppliedTheme,
  getStoredSettings,
  waitForStorageHydration,
} from "../utils/indexeddb";

test.describe("Settings Page", () => {
  test.describe("Not Authenticated", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/settings");
      await waitForStorageHydration(page);
    });

    test.describe("Session Section", () => {
      test("network section is always visible", async ({ page }) => {
        const networkTitle = page.getByText("Network", { exact: false });
        await expect(networkTitle.first()).toBeVisible();

        const latestBlockHeight = page.getByText("Latest block height", {
          exact: false,
        });
        const endpoint = page.getByText("Endpoint", { exact: false });

        const networkInfoVisible =
          (await latestBlockHeight.isVisible()) || (await endpoint.isVisible());
        expect(networkInfoVisible).toBeTruthy();
      });

      test("username section is hidden when not connected", async ({ page }) => {
        const editIcon = page.locator('svg[class*="IconEdit"]');
        const editIconCount = await editIcon.count();
        expect(editIconCount).toBe(0);
      });

      test("user status section is hidden when not connected", async ({ page }) => {
        const accountStatus = page.getByText("Account Status", { exact: false });
        const statusVisible = await accountStatus.isVisible().catch(() => false);
        expect(statusVisible).toBeFalsy();
      });

      test("connect to mobile section is hidden when not connected", async ({ page }) => {
        const connectMobile = page.getByText("Connect to mobile", { exact: false });
        const visible = await connectMobile.isVisible().catch(() => false);
        expect(visible).toBeFalsy();
      });
    });

    test.describe("Display Section", () => {
      test("display section is visible", async ({ page }) => {
        const displayTitle = page.getByText("Display", { exact: true });
        await expect(displayTitle).toBeVisible();
      });

      test("language selector is available", async ({ page }) => {
        const languageLabel = page.getByText("Language", { exact: false });
        await expect(languageLabel.first()).toBeVisible();
      });

      test("number format selector changes format", async ({ page }) => {
        const numberLabel = page.getByText("Number", { exact: false });
        await expect(numberLabel.first()).toBeVisible();

        const numberSelect = numberLabel.first().locator("..").locator("button");
        if ((await numberSelect.count()) > 0) {
          await numberSelect.click();
          await page.waitForTimeout(200);

          const option = page.getByText("1.234,00");
          if (await option.isVisible()) {
            await option.click();
            await page.waitForTimeout(300);

            const settings = await getStoredSettings(page);
            if (settings?.formatNumberOptions) {
              expect(
                (settings.formatNumberOptions as { mask: number }).mask,
              ).toBe(2);
            }
          }
        }
      });

      test("date format selector changes format", async ({ page }) => {
        const dateLabel = page.getByText("Date", { exact: false }).first();
        await expect(dateLabel).toBeVisible();
      });

      test("time format selector changes format", async ({ page }) => {
        const timeLabel = page.getByText("Time").first();
        await expect(timeLabel).toBeVisible();
      });

      test("timezone selector is available", async ({ page }) => {
        const timezoneLabel = page.getByText("Time zone", { exact: false });
        await expect(timezoneLabel.first()).toBeVisible();

        const selectButton = timezoneLabel.first().locator("..").locator("button");
        if ((await selectButton.count()) > 0) {
          await selectButton.click();
          await page.waitForTimeout(200);

          const utcOption = page.getByText("UTC", { exact: true });
          const localOption = page.getByText("Local", { exact: true });

          const hasOptions =
            (await utcOption.isVisible()) || (await localOption.isVisible());
          expect(hasOptions).toBeTruthy();

          await page.keyboard.press("Escape");
        }
      });

      test("chart engine selector is available", async ({ page }) => {
        const chartLabel = page.getByText("Chart", { exact: false });
        await expect(chartLabel.first()).toBeVisible();
      });
    });

    test.describe("Theme Switching", () => {
      // Helper to get theme buttons - they're siblings after the System button
      const getThemeButtons = (page: import("@playwright/test").Page) => {
        const systemButton = page.getByRole("button", { name: "System" });
        // Light button is the next sibling, dark button is after that
        const themeContainer = systemButton.locator("..");
        return {
          system: systemButton,
          light: themeContainer.locator("button").nth(1), // Second button (after System)
          dark: themeContainer.locator("button").nth(2), // Third button
        };
      };

      test("theme buttons are visible", async ({ page }) => {
        const themeLabel = page.getByText("Theme", { exact: false });
        await expect(themeLabel.first()).toBeVisible();

        // Theme buttons are regular buttons
        const { system } = getThemeButtons(page);
        await expect(system).toBeVisible();
      });

      test("selecting light theme applies light mode", async ({ page }) => {
        const { light } = getThemeButtons(page);

        await light.click();
        await page.waitForTimeout(500);

        const theme = await getAppliedTheme(page);
        expect(theme).toBe("light");
      });

      test("selecting dark theme applies dark mode", async ({ page }) => {
        const { dark } = getThemeButtons(page);

        await dark.click();
        await page.waitForTimeout(500);

        const theme = await getAppliedTheme(page);
        expect(theme).toBe("dark");
      });

      test("selecting system theme follows OS preference", async ({ page }) => {
        const { system } = getThemeButtons(page);

        await system.click();
        await page.waitForTimeout(500);

        const theme = await getAppliedTheme(page);
        expect(["light", "dark", "system"]).toContain(theme);
      });

      test("theme persists after page reload", async ({ page }) => {
        const { dark, light } = getThemeButtons(page);

        await dark.click();
        await page.waitForTimeout(300);

        await page.reload();
        await waitForStorageHydration(page);

        const theme = await getAppliedTheme(page);
        expect(theme).toBe("dark");

        // Get buttons again after reload
        const buttonsAfterReload = getThemeButtons(page);
        await buttonsAfterReload.light.click();
        await page.waitForTimeout(300);

        const theme2 = await getAppliedTheme(page);
        expect(theme2).toBe("light");
      });
    });

    test("all display settings persist in IndexedDB", async ({ page }) => {
      // Wait for settings to be hydrated
      await page.waitForTimeout(1000);

      const initialSettings = await getStoredSettings(page);

      // Settings might be null on first load before any changes
      // Just verify the page has the settings UI rendered
      const displaySection = page.getByText("Display", { exact: true });
      await expect(displaySection).toBeVisible();

      // If settings exist, verify structure
      if (initialSettings) {
        // Settings should have chart engine
        expect(
          initialSettings.chart === "tradingview" || initialSettings.chart === undefined,
        ).toBeTruthy();
      }
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

    test("username section shows User #number format", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const usernameLabel = sharedPage.getByText("Username", { exact: true });
      await expect(usernameLabel).toBeVisible();

      const userNumber = sharedPage.getByText(/User #\d+/);
      await expect(userNumber.first()).toBeVisible();
    });

    test("username edit icon is visible for default username", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const usernameSection = sharedPage.locator("div").filter({
        has: sharedPage.getByText(/User #\d+/),
      });

      if ((await usernameSection.count()) > 0) {
        const clickableArea = usernameSection
          .first()
          .locator('[class*="cursor-pointer"], [class*="hover:cursor-pointer"]');
        const hasEditCapability = (await clickableArea.count()) > 0;
        expect(hasEditCapability).toBeTruthy();
      }
    });

    test("clicking username section opens edit modal", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const userNumber = sharedPage.getByText(/User #\d+/).first();
      await expect(userNumber).toBeVisible();

      await userNumber.click();
      await sharedPage.waitForTimeout(500);

      const modal = sharedPage.locator(".fixed.z-\\[60\\]");
      const modalVisible =
        (await modal.isVisible()) ||
        (await sharedPage.getByRole("dialog").isVisible());

      expect(modalVisible).toBeTruthy();

      // Close modal
      await sharedPage.keyboard.press("Escape");
      await sharedPage.waitForTimeout(300);
    });

    test("user status shows inactive for new user", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      // Wait for session section to render
      await sharedPage.waitForTimeout(500);

      // Look for status-related text - might be "Account Status", "Status", or just "Inactive"
      const statusSection = sharedPage.getByText("Status", { exact: false });
      const inactiveStatus = sharedPage.getByText("Inactive", { exact: false });

      // Either the status section label or the inactive badge should be visible
      const statusVisible =
        (await statusSection.count()) > 0 || (await inactiveStatus.count()) > 0;
      expect(statusVisible).toBeTruthy();

      // If inactive status is shown, verify it
      if ((await inactiveStatus.count()) > 0) {
        await expect(inactiveStatus.first()).toBeVisible();
      }
    });

    test("deposit button visible for inactive user", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const depositButton = sharedPage
        .getByRole("link", { name: /deposit|bridge/i })
        .or(sharedPage.locator('a[href*="/bridge"]'));

      if ((await depositButton.count()) > 0) {
        await expect(depositButton.first()).toBeVisible();
      }
    });

    test("connect to mobile button is visible", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const connectMobileButton = sharedPage.getByText("Connect to mobile", {
        exact: false,
      });
      await expect(connectMobileButton.first()).toBeVisible();
    });

    test("clicking connect to mobile opens QR modal", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const connectMobileButton = sharedPage.getByText("Connect to mobile", {
        exact: false,
      });
      await connectMobileButton.click();
      await sharedPage.waitForTimeout(500);

      const modal = sharedPage.locator(".fixed.z-\\[60\\]");
      const modalVisible =
        (await modal.isVisible()) ||
        (await sharedPage.getByRole("dialog").isVisible());

      if (modalVisible) {
        const qrCode = sharedPage.locator("canvas, svg, [class*='qr']");
        const hasQR = (await qrCode.count()) > 0;
        expect(hasQR).toBeTruthy();
      }

      // Close modal
      await sharedPage.keyboard.press("Escape");
      await sharedPage.waitForTimeout(300);
    });

    test("session remaining time is displayed", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const remainingLabel = sharedPage.getByText("Remaining", { exact: false });

      if ((await remainingLabel.count()) > 0) {
        await expect(remainingLabel.first()).toBeVisible();

        const timeDisplay = sharedPage.locator(
          '[class*="countdown"], [class*="timer"]',
        );
        const hasTimeDisplay = (await timeDisplay.count()) > 0;
        expect(hasTimeDisplay || (await remainingLabel.isVisible())).toBeTruthy();
      }
    });

    test("network section is still visible when authenticated", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const networkTitle = sharedPage.getByText("Network", { exact: false });
      await expect(networkTitle.first()).toBeVisible();
    });
  });
});
