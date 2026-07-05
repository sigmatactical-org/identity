import { test, expect } from "@playwright/test";

import { configuration } from "../conf.ts";
import { loginUser, logoutViaExampleApp } from "../helper.ts";

test.describe("example app login", () => {
  test("completes OIDC login and logout", async ({ context, page }) => {
    const conf = await configuration();
    await page.goto(`${conf.baseUrl}/exampleapp/`);
    await expect(page.getByRole("heading", { name: "Actions" })).toBeVisible();
    await expect(page.getByText("not authenticated")).toBeVisible();

    await Promise.all([
      page.waitForResponse(
        (response) =>
          response.url().includes("/auth/login") &&
          (response.status() === 303 || response.status() === 302),
      ),
      page.getByRole("button", { name: "Login" }).click(),
    ]);
    const cookies = await context.cookies();
    expect(cookies.map((cookie) => cookie.name)).toContain("identity.sid");

    await loginUser(page, "user1");
    await expect(page.locator("#loginStatus")).toContainText("authenticated");

    await logoutViaExampleApp(page);
  });
});
