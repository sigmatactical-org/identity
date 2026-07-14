import { test, expect } from "@playwright/test";

import { configuration } from "../conf.ts";
import {
  authenticatedProxyRequest,
  fetchCsrfToken,
  loginViaExampleApp,
} from "../helper.ts";

test.describe("proxy API", () => {
  test("rejects unauthenticated requests", async ({ request }) => {
    const conf = await configuration();
    const response = await request.post(`${conf.baseUrl}/api/echorequest`, {
      headers: { "X-CSRF-TOKEN": "not-a-real-token" },
    });

    // Authentication is checked before CSRF: anonymous callers get 401
    // (introspection hardening), not the CSRF 403.
    expect(response.status()).toBe(401);
    expect(await response.text()).toContain("Authentication required");
  });
});

test.describe("proxy routing rules", () => {
  test.beforeEach(async ({ page }) => {
    await loginViaExampleApp(page);
  });

  test("routes /api/num via RULE_A", async ({ page, request }) => {
    const cookies = await page.context().cookies();
    const csrfToken = await fetchCsrfToken(request, cookies);
    const response = await authenticatedProxyRequest(
      request,
      cookies,
      csrfToken,
      "/num/extra",
      "rule-a",
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.text();
    expect(body).toContain("POST request at /rule/a/extra");
    expect(body).toContain("authorization: Bearer eyJ");
  });

  test("routes /api/foo via RULE_FOO", async ({ page, request }) => {
    const cookies = await page.context().cookies();
    const csrfToken = await fetchCsrfToken(request, cookies);
    const response = await authenticatedProxyRequest(
      request,
      cookies,
      csrfToken,
      "/foo/suffix",
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.text();
    expect(body).toContain("POST request at /foo-rule/abc/suffix");
  });

  test("routes /api/bar/me/no via RULE_BAR", async ({ page, request }) => {
    const cookies = await page.context().cookies();
    const csrfToken = await fetchCsrfToken(request, cookies);
    const response = await authenticatedProxyRequest(
      request,
      cookies,
      csrfToken,
      "/bar/me/no/tail",
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.text();
    expect(body).toContain("POST request at /bar/bar/bar/tail");
  });
});
