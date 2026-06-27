import { test, expect } from "@playwright/test";

import { loginUser } from "../helper.ts";

test.describe("example app proxy", () => {
  test("requires CSRF token and attaches JWT to proxied requests", async ({
    page,
  }) => {
    await page.goto("/exampleapp/");
    await expect(page.locator("#loginStatus")).toContainText("not authenticated");

    await expect(page.locator("#csrftoken")).toBeEmpty();
    await page.getByRole("button", { name: "Echo request" }).click();
    await expect(page.locator("#echoresponse")).toContainText(
      "Missing or invalid CSRF token",
    );

    await page.getByRole("button", { name: "Login" }).click();
    await loginUser(page, "user1");
    await expect(page.locator("#loginStatus")).toContainText("authenticated");

    await expect(page.locator("#csrftoken")).toBeEmpty();
    await page.getByRole("button", { name: "Echo request" }).click();
    await expect(page.locator("#echoresponse")).toContainText(
      "Missing or invalid CSRF token",
    );

    await page.getByRole("button", { name: "CSRF token" }).click();
    await expect(page.locator("#csrftoken")).not.toBeEmpty();

    await page.getByRole("button", { name: "Echo request" }).click();
    await expect(page.locator("#echoresponse")).toContainText(
      "POST request at /echorequest",
    );
    await expect(page.locator("#echoresponse")).toContainText(
      "authorization: Bearer eyJ",
    );

    await page.getByRole("button", { name: "Logout" }).click();
    await expect(page.locator("#loginStatus")).toContainText("not authenticated");
    await expect(page.locator("#csrftoken")).toBeEmpty();
  });
});
