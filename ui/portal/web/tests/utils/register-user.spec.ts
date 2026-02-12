import { expect, test } from "@playwright/test";
import { registerUser } from "./registerUser";

test("User registration shows account info in header", async ({ page }) => {
  await registerUser(page);

  // Verify the header button shows account info instead of "Log In"
  const headerButton = page.locator("[dng-connect-button]");
  await expect(headerButton).toContainText("Account #");
});
