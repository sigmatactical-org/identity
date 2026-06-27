import type { CsrfTokenResponse } from "./csrf-token-response";
import { csrfToken } from "./csrf-token";
import { requireElement } from "./require-element";

export const requestCsrfToken = {
  attach(): void {
    requireElement<HTMLButtonElement>("requestcsrftoken").onclick = () => {
      fetch("/auth/csrftoken", { method: "POST" })
        .then((response) => response.json() as Promise<CsrfTokenResponse>)
        .then((data) => {
          csrfToken.set(data.token);
          requireElement<HTMLElement>("csrftoken").innerText = csrfToken.get();
        });
    };
  },
};
