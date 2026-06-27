import { test, expect } from "@playwright/test";

import { loginViaExampleApp } from "../helper.ts";

test.describe("token refresh", () => {
  test("keeps the session authenticated", async ({ page }) => {
    await loginViaExampleApp(page);
    await expect(page.locator("#loginStatus")).toContainText("authenticated");

    await page.getByRole("button", { name: "Refresh" }).click();

    await expect(page.locator("#loginStatus")).toContainText("authenticated", {
      timeout: 10_000,
    });
    await expect(page.locator("#loginStatus")).toContainText("exp:");
  });
});
