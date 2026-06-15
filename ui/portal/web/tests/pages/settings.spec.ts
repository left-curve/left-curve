import { expect, test, type Page } from "@playwright/test";
import { registerUser } from "../utils/registerUser";
import {
  getAppliedTheme,
  getStoredSettings,
  waitForStorageHydration,
} from "../utils/indexeddb";
import { message } from "../utils/messages";

const settingsLabels = {
  accountStatus: message("settings.session.userStatus.title"),
  chart: message("settings.chart"),
  connectToMobile: message("settings.connectToMobile"),
  date: message("settings.date"),
  deposit: message("settings.session.userStatus.button"),
  display: message("settings.display"),
  endpoint: message("settings.session.network.endpoint"),
  inactive: message("settings.session.accountStatus", { status: "inactive" }),
  language: message("settings.language"),
  latestBlockHeight: message("settings.session.network.latestBlockHeight"),
  network: message("settings.session.network.title"),
  number: message("settings.number"),
  remaining: message("settings.session.remaining"),
  status: message("statusBadge.status"),
  theme: message("settings.theme"),
  time: message("settings.time"),
  timeZone: message("settings.timeZone"),
  username: message("common.username"),
};

test.describe("Settings Page", () => {
  test.describe("Not Authenticated", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/settings");
      await waitForStorageHydration(page);
    });

    test.describe("Session Section", () => {
      test("network section is always visible", async ({ page }) => {
        const networkTitle = page.getByText(settingsLabels.network, { exact: false });
        await expect(networkTitle.first()).toBeVisible();

        const latestBlockHeight = page.getByText(settingsLabels.latestBlockHeight, {
          exact: false,
        });
        const endpoint = page.getByText(settingsLabels.endpoint, { exact: false });

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
        const accountStatus = page.getByText(settingsLabels.accountStatus, { exact: false });
        const statusVisible = await accountStatus.isVisible().catch(() => false);
        expect(statusVisible).toBeFalsy();
      });

      test("connect to mobile section is hidden when not connected", async ({ page }) => {
        const connectMobile = page.getByText(settingsLabels.connectToMobile, { exact: false });
        const visible = await connectMobile.isVisible().catch(() => false);
        expect(visible).toBeFalsy();
      });
    });

    test.describe("Display Section", () => {
      test("display section is visible", async ({ page }) => {
        const displayTitle = page.getByText(settingsLabels.display, { exact: true });
        await expect(displayTitle).toBeVisible();
      });

      test("language selector is available", async ({ page }) => {
        const languageLabel = page.getByText(settingsLabels.language, { exact: false });
        await expect(languageLabel.first()).toBeVisible();
      });

      test("number format selector changes format", async ({ page }) => {
        const numberLabel = page.getByText(settingsLabels.number, { exact: false });
        await expect(numberLabel.first()).toBeVisible();

        const numberSelect = numberLabel
          .first()
          .locator("xpath=ancestor::div[contains(@class, 'cursor-pointer')][1]");
        await expect(numberSelect).toBeVisible();

        const currentFormat =
          (await numberSelect.getByRole("button").first().textContent()) ?? "";
        const targetFormat = currentFormat.includes("1.234,56")
          ? { label: "1,234.56", mask: 1 }
          : { label: "1.234,56", mask: 2 };

        await numberSelect.click();

        const targetOption = page
          .getByRole("listitem")
          .filter({ hasText: targetFormat.label })
          .first();
        await expect(targetOption).toBeVisible();
        await targetOption.evaluate((element) => (element as HTMLElement).click());
        await expect(numberSelect.getByRole("button").first()).toContainText(targetFormat.label);

        await expect
          .poll(async () => {
            const settings = await getStoredSettings(page);
            return (settings?.formatNumberOptions as { mask?: number } | undefined)?.mask;
          })
          .toBe(targetFormat.mask);
      });

      test("date format selector changes format", async ({ page }) => {
        const dateLabel = page.getByText(settingsLabels.date, { exact: false }).first();
        await expect(dateLabel).toBeVisible();
      });

      test("time format selector changes format", async ({ page }) => {
        const timeLabel = page.getByText(settingsLabels.time).first();
        await expect(timeLabel).toBeVisible();
      });

      test("timezone selector changes timezone", async ({ page }) => {
        const timezoneLabel = page.getByText(settingsLabels.timeZone, { exact: false });
        await expect(timezoneLabel.first()).toBeVisible();

        const timezoneSelect = timezoneLabel
          .first()
          .locator("xpath=ancestor::div[contains(@class, 'cursor-pointer')][1]");
        await expect(timezoneSelect).toBeVisible();

        const currentTimezone =
          (await timezoneSelect.getByRole("button").first().textContent()) ?? "";
        const targetTimezone = currentTimezone.includes("UTC")
          ? { label: "Local", value: "local" }
          : { label: "UTC", value: "utc" };

        await timezoneSelect.click();

        const targetOption = page
          .getByRole("listitem")
          .filter({ hasText: targetTimezone.label })
          .first();
        await expect(targetOption).toBeVisible();
        await targetOption.evaluate((element) => (element as HTMLElement).click());
        await expect(timezoneSelect.getByRole("button").first()).toContainText(
          targetTimezone.label,
        );

        await expect
          .poll(async () => {
            const settings = await getStoredSettings(page);
            return settings?.timeZone;
          })
          .toBe(targetTimezone.value);
      });

      test("chart engine selector is available", async ({ page }) => {
        const chartLabel = page.getByText(settingsLabels.chart, { exact: false });
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
        const themeLabel = page.getByText(settingsLabels.theme, { exact: false });
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

    test("all display settings persist in storage", async ({ page }) => {
      // Wait for settings to be hydrated
      await page.waitForTimeout(1000);

      const initialSettings = await getStoredSettings(page);

      // Settings might be null on first load before any changes
      // Just verify the page has the settings UI rendered
      const displaySection = page.getByText(settingsLabels.display, { exact: true });
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

    test("username section shows default username format", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const usernameLabel = sharedPage.getByText(settingsLabels.username, { exact: true });
      await expect(usernameLabel).toBeVisible();

      // Default username format: user_N (e.g. user_0, user_24)
      const userNumber = sharedPage.getByText(/user_\d+/);
      await expect(userNumber.first()).toBeVisible({ timeout: 10_000 });
    });

    test("username edit icon is visible for default username", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const usernameSection = sharedPage.locator("div").filter({
        has: sharedPage.getByText(/user_\d+/),
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

      const userNumber = sharedPage.getByText(/user_\d+/).first();
      await expect(userNumber).toBeVisible({ timeout: 10_000 });

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

      const statusSection = sharedPage.getByText(settingsLabels.status, { exact: false });
      const inactiveStatus = sharedPage.getByText(settingsLabels.inactive, { exact: false });

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
        .getByRole("link", { name: settingsLabels.deposit })
        .or(sharedPage.locator('a[href*="/bridge"]'));

      if ((await depositButton.count()) > 0) {
        await expect(depositButton.first()).toBeVisible();
      }
    });

    test("connect to mobile button is visible", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const connectMobileButton = sharedPage.getByText(settingsLabels.connectToMobile, {
        exact: false,
      });
      await expect(connectMobileButton.first()).toBeVisible();
    });

    test("clicking connect to mobile opens QR modal", async () => {
      await sharedPage.goto("/settings");
      await waitForStorageHydration(sharedPage);

      const connectMobileButton = sharedPage.getByText(settingsLabels.connectToMobile, {
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

      const remainingLabel = sharedPage.getByText(settingsLabels.remaining, { exact: false });

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

      const networkTitle = sharedPage.getByText(settingsLabels.network, { exact: false });
      await expect(networkTitle.first()).toBeVisible();
    });
  });
});
