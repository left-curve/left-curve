import { expect, test } from "@playwright/test";
import { message } from "./messages";
import { registerUser } from "./registerUser";

test("User registration shows account info in header", async ({ page }) => {
  await registerUser(page);

  // Verify the header button shows account info.
  const headerButton = page.locator("[dng-connect-button]");
  await expect(headerButton).toContainText(`${message("common.account")} #`);
});
