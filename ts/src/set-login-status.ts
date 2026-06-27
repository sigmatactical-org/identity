import type { AuthStatus } from "./auth-status";
import { csrfToken } from "./csrf-token";
import { requireElement } from "./require-element";

export function setLoginStatus(data: AuthStatus): void {
  const loginStatus = requireElement<HTMLElement>("loginStatus");
  loginStatus.innerHTML = data.authenticated
    ? `authenticated (exp: ${data.expires_in}, refexp: ${data.refresh_expires_in})`
    : "not authenticated";
  requireElement<HTMLElement>("csrftoken").innerText = csrfToken.get();
}
