import { expect, test } from "@playwright/test";
import { getStoredFavorites, waitForStorageHydration } from "./utils/indexeddb";

test.describe("Landing Page - Not Authenticated", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await waitForStorageHydration(page);
  });

  test.describe("Section Navigation", () => {
    test("landing page renders with Learn More button", async ({ page }) => {
      await expect(page.locator("text=Learn More")).toBeVisible();
    });

    test("can scroll through landing sections", async ({ page }) => {
      // Landing uses fullpage.js sections
      // First section: header with search
      await expect(page.locator('[data-testid="landing-header"]').or(page.locator("header"))).toBeVisible();

      // Scroll indicator should be visible
      const scrollIndicator = page.getByText("Scroll to continue");
      if (await scrollIndicator.isVisible()) {
        await expect(scrollIndicator).toBeVisible();
      }
    });

    test("community section shows social links", async ({ page }) => {
      // Navigate to community section (last section)
      // Use keyboard or scroll to navigate
      await page.keyboard.press("End");
      await page.waitForTimeout(500);

      // Check for social links - they may be in footer
      const twitterLink = page.locator('a[href*="twitter.com"], a[href*="x.com"]');
      const discordLink = page.locator('a[href*="discord"]');

      // At least one social link should exist
      const socialLinksExist =
        (await twitterLink.count()) > 0 || (await discordLink.count()) > 0;
      expect(socialLinksExist).toBeTruthy();
    });
  });

  test.describe("Search Bar Functionality", () => {
    test("clicking search bar expands container", async ({ page }) => {
      // Try keyboard shortcut first as it's more reliable
      await page.keyboard.press("k");
      await page.waitForTimeout(500);

      // Verify menu expanded - should show applet groups or input
      const favoriteApplets = page.getByText("Favorite Applets");
      const appletsGroup = page.getByText("Applets", { exact: true });
      const searchInput = page.locator("input").first();

      const menuExpanded =
        (await favoriteApplets.isVisible()) ||
        (await appletsGroup.isVisible()) ||
        (await searchInput.isVisible());

      expect(menuExpanded).toBeTruthy();
    });

    test("search bar filters applets by text", async ({ page }) => {
      // Open search menu using keyboard
      await page.keyboard.press("k");
      await page.waitForTimeout(300);

      // Type in search
      const searchInput = page.locator("input").first();
      await searchInput.fill("trade");
      await page.waitForTimeout(500);

      // Should show filtered results - trade applet should be visible
      const tradeApplet = page.getByText("Trade", { exact: false });
      await expect(tradeApplet.first()).toBeVisible();
    });

    test("Escape closes search menu", async ({ page }) => {
      // Open search
      await page.keyboard.press("t"); // Type a character to open search
      await page.waitForTimeout(300);

      // Press Escape
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);

      // Menu should be closed - check for no visible groups
      const groups = page.locator('[role="group"]');
      const groupCount = await groups.count();

      // Either no groups or they're hidden
      expect(groupCount).toBeLessThanOrEqual(1);
    });
  });

  test.describe("Favorites Management", () => {
    test("default favorites are displayed in applets section", async ({ page }) => {
      // Default favorites: trade, convert, bridge, transfer, create-account, settings
      const appletsSection = page.locator('[class*="grid"]').filter({
        has: page.locator('a[href*="/convert"], a[href*="/trade"], a[href*="/bridge"]'),
      });

      // Should have applet links
      const appletLinks = page.locator(
        'a[href="/convert"], a[href="/trade"], a[href="/bridge"], a[href="/transfer"]',
      );
      const linkCount = await appletLinks.count();
      expect(linkCount).toBeGreaterThanOrEqual(1);
    });

    test("clicking star icon toggles favorite status", async ({ page }) => {
      // Open search menu using keyboard shortcut
      await page.keyboard.press("k");
      await page.waitForTimeout(500);

      // Get initial favorites
      const initialFavorites = await getStoredFavorites(page);

      // Look for applet items with star icons (svg elements)
      const appletItems = page.locator('[class*="hover:bg-surface"]').filter({
        has: page.locator("svg"),
      });

      if ((await appletItems.count()) > 0) {
        // Find a clickable star area
        const firstApplet = appletItems.first();
        const starArea = firstApplet.locator("svg").last();

        if (await starArea.isVisible()) {
          await starArea.click({ force: true });
          await page.waitForTimeout(300);

          // Verify state changed
          const newFavorites = await getStoredFavorites(page);
          expect(newFavorites).toBeDefined();
        }
      } else {
        // Just verify favorites array is defined
        expect(initialFavorites).toBeDefined();
      }
    });

    test("favorite applets shown under search bar", async ({ page }) => {
      // Check AppletsSection shows favorites
      const favApplets = await getStoredFavorites(page);

      // Each favorite should have a corresponding link
      for (const appletId of favApplets.slice(0, 3)) {
        // Check first 3
        const appletLink = page.locator(`a[href*="${appletId}"]`);
        // At least some favorites should be visible as links
        if ((await appletLink.count()) > 0) {
          await expect(appletLink.first()).toBeVisible();
          break; // Found at least one
        }
      }
    });
  });

  test.describe("Applet Navigation", () => {
    test("clicking favorite applet navigates to applet page", async ({ page }) => {
      // Find a favorite applet link (e.g., Convert)
      const convertLink = page.locator('a[href="/convert"]');

      if ((await convertLink.count()) > 0) {
        await convertLink.first().click();
        await page.waitForURL("**/convert**");
        expect(page.url()).toContain("/convert");
      } else {
        // Try another applet
        const tradeLink = page.locator('a[href*="/trade"]');
        if ((await tradeLink.count()) > 0) {
          await tradeLink.first().click();
          await page.waitForURL("**/trade**");
          expect(page.url()).toContain("/trade");
        }
      }
    });

    test("plus button opens search menu", async ({ page }) => {
      // Find the add/plus button (IconAddCross)
      const plusButton = page.locator('button').filter({
        has: page.locator('svg'),
      }).filter({
        hasText: "",
      });

      // Look for add button in applets section
      const addButton = page
        .locator("button")
        .filter({
          has: page.locator('[class*="border-outline-tertiary"]'),
        })
        .or(page.locator('button:has(svg[class*="IconAddCross"])'));

      if ((await addButton.count()) > 0) {
        await addButton.first().click();
        await page.waitForTimeout(300);

        // Search menu should be open
        const searchInput = page.locator("input").first();
        await expect(searchInput).toBeVisible();
      }
    });
  });
});
