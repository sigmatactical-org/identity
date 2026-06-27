import { requireElement } from "./require-element";

export const refresh = {
  attach(): void {
    requireElement<HTMLButtonElement>("refresh").onclick = () => {
      fetch("/auth/refresh", { method: "POST" })
        .then((response) => response.json())
        .then((refreshData) => console.log(refreshData));
    };
  },
};
