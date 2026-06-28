import { expect, type Cookie, type Page, type APIRequestContext } from "@playwright/test";

import { configuration } from "./conf.ts";

export type KnownUsers = "user1";

const credentials: Record<KnownUsers, { username: string; password: string }> = {
  user1: { username: "user1", password: "user1" },
};

export const loginUser = async (page: Page, user: KnownUsers) => {
  const { username, password } = credentials[user];
  await page.getByRole("textbox", { name: "Username or email" }).fill(username);
  await page.getByRole("textbox", { name: "Password" }).fill(password);
  await page.getByRole("button", { name: "Sign In" }).click();
};

export const loginViaExampleApp = async (page: Page, user: KnownUsers = "user1") => {
  const conf = await configuration();
  await page.goto(`${conf.baseUrl}/exampleapp/`);
  await page.getByRole("button", { name: "Login" }).click();
  await page.getByRole("textbox", { name: "Username or email" }).waitFor();
  await loginUser(page, user);
  await expect(page.locator("#loginStatus")).toContainText("authenticated", {
    timeout: 15_000,
  });
};

const exampleAppUrl = /\/exampleapp\/?/;

export const logoutViaExampleApp = async (page: Page) => {
  await page.getByRole("button", { name: "Logout" }).click();

  // Keycloak may require a second confirmation when id_token_hint is missing/invalid.
  await page
    .getByRole("button", { name: "Logout" })
    .click({ timeout: 5_000 })
    .catch(() => {});

  try {
    await page.waitForURL(exampleAppUrl, { timeout: 30_000 });
  } catch {
    const backToApp = page.getByRole("link", { name: /back to application/i });
    if (await backToApp.isVisible().catch(() => false)) {
      await backToApp.click();
    }
    await page.waitForURL(exampleAppUrl, { timeout: 30_000 });
  }

  await expect(page.locator("#loginStatus")).toContainText("not authenticated", {
    timeout: 15_000,
  });
};

export const cookieHeader = (cookies: Cookie[]) =>
  cookies.map((cookie) => `${cookie.name}=${cookie.value}`).join("; ");

export const fetchCsrfToken = async (
  request: APIRequestContext,
  cookies: Cookie[],
) => {
  const conf = await configuration();
  const response = await request.post(`${conf.baseUrl}/auth/csrftoken`, {
    headers: { cookie: cookieHeader(cookies) },
  });
  expect(response.ok()).toBeTruthy();
  const body = (await response.json()) as { token: string };
  return body.token;
};

export const authenticatedProxyRequest = async (
  request: APIRequestContext,
  cookies: Cookie[],
  csrfToken: string,
  path: string,
  data?: string,
) => {
  const conf = await configuration();
  return request.post(`${conf.baseUrl}/api${path}`, {
    headers: {
      cookie: cookieHeader(cookies),
      "X-CSRF-TOKEN": csrfToken,
    },
    data,
  });
};
