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

  // Keycloak may show a confirmation step when no id_token_hint is sent.
  await page
    .getByRole("button", { name: "Logout" })
    .click({ timeout: 5_000 })
    .catch(() => {});

  await page.waitForURL(
    (url) =>
      exampleAppUrl.test(url.pathname) ||
      url.pathname === "/auth/logoutcallback",
    { timeout: 45_000, waitUntil: "commit" },
  );

  if (!exampleAppUrl.test(new URL(page.url()).pathname)) {
    await page.waitForURL(exampleAppUrl, { timeout: 15_000, waitUntil: "commit" });
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
