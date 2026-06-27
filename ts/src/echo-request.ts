import { csrfToken } from "./csrf-token";
import { requireElement } from "./require-element";

export const echoRequest = {
  attach(): void {
    requireElement<HTMLButtonElement>("echorequest").onclick = () => {
      const headers = new Headers();
      headers.append("X-CSRF-TOKEN", csrfToken.get());
      fetch("/api/echorequest", {
        method: "POST",
        headers,
        body: JSON.stringify({
          usermessage: requireElement<HTMLTextAreaElement>("userinput").value,
        }),
      })
        .then((response) => response.text())
        .then((responseText) => {
          requireElement<HTMLElement>("echoresponse").innerText = responseText;
        });
    };
  },
};
