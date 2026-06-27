import { queryString } from "./query-string";
import { requireElement } from "./require-element";

export const logout = {
  attach(): void {
    requireElement<HTMLButtonElement>("logout").onclick = () => {
      const oidCallbackUrl = `${window.location.origin}/auth/logoutcallback`;
      const appCallbackUrl = `${window.location.origin}/exampleapp/`;

      window.location.href = `/auth/logout?${queryString({
        redirect_uri: oidCallbackUrl,
        app_uri: appCallbackUrl,
      })}`;
    };
  },
};
