import type { AuthStatus } from "./auth-status";
import { requireElement } from "./require-element";
import { setLoginStatus } from "./set-login-status";

export const isLoggedIn = {
  poll(): void {
    fetch("/auth/status")
      .then((response) => response.json() as Promise<AuthStatus>)
      .then(setLoginStatus)
      .catch(() => {
        requireElement<HTMLElement>("loginStatus").innerHTML = "not authenticated";
      });
  },

  startPolling(intervalMs = 1000): void {
    setInterval(() => isLoggedIn.poll(), intervalMs);
  },
};
