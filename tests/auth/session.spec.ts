import { test, expect } from "@playwright/test";

import { configuration } from "../conf.ts";

test.describe("auth login", () => {
  test("rejects disallowed app_uri", async ({ request }) => {
    const conf = await configuration();
    const response = await request.get(
      `${conf.baseUrl}/auth/login?${new URLSearchParams({
        app_uri: "https://evil.example.com/callback",
        redirect_uri: `${conf.baseUrl}/auth/callback`,
        scope: "openid",
        state: "e2e-invalid-app-uri",
      })}`,
    );

    expect(response.status()).toBe(400);
    expect(await response.text()).toContain("Invalid app_uri");
  });
});

test.describe("auth status", () => {
  test("reports unauthenticated without a session", async ({ request }) => {
    const conf = await configuration();
    const response = await request.get(`${conf.baseUrl}/auth/status`);

    expect(response.ok()).toBeTruthy();
    await expect(response.json()).resolves.toMatchObject({ authenticated: false });
  });

  test("reports authenticated after OIDC login", async ({ page, request }) => {
    const conf = await configuration();
    await page.goto(`${conf.baseUrl}/exampleapp/`);
    await Promise.all([
      page.waitForURL(/\/auth\/login|keycloak|8101/),
      page.getByRole("button", { name: "Login" }).click(),
    ]);
    await page.getByRole("textbox", { name: "Username or email" }).fill("user1");
    await page.getByRole("textbox", { name: "Password" }).fill("user1");
    await page.getByRole("button", { name: "Sign In" }).click();
    await expect(page.locator("#loginStatus")).toContainText("authenticated", {
      timeout: 15_000,
    });

    const cookies = await page.context().cookies();
    const response = await request.get(`${conf.baseUrl}/auth/status`, {
      headers: {
        cookie: cookies.map((c) => `${c.name}=${c.value}`).join("; "),
      },
    });

    expect(response.ok()).toBeTruthy();
    const body = (await response.json()) as {
      authenticated: boolean;
      expires_in?: number;
    };
    expect(body.authenticated).toBe(true);
    expect(body.expires_in).toBeGreaterThan(0);
  });
});
