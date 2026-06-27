import { queryString } from "./query-string";
import { requireElement } from "./require-element";

export const login = {
  attach(): void {
    requireElement<HTMLButtonElement>("login").onclick = () => {
      const oidCallbackUrl = `${window.location.origin}/auth/callback`;
      const appCallbackUrl = `${window.location.origin}/exampleapp/`;
      const state = `${Math.random().toString(36).substring(2, 15)}-appstate-${Math.random().toString(36).substring(2, 15)}`;

      sessionStorage.setItem("state", state);

      window.location.href = `/auth/login?${queryString({
        scope: "openid profile email",
        redirect_uri: oidCallbackUrl,
        app_uri: appCallbackUrl,
        state,
      })}`;
    };
  },
};
