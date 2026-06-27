interface AuthStatus {
  authenticated: boolean;
  expires_in?: number;
  refresh_expires_in?: number;
}

interface CsrfTokenResponse {
  token: string;
}

let csrftoken = "";

function requireElement<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) {
    throw new Error(`Missing #${id}`);
  }
  return el as T;
}

function setLoginStatus(data: AuthStatus): void {
  const loginStatus = requireElement<HTMLElement>("loginStatus");
  loginStatus.innerHTML = data.authenticated
    ? `authenticated (exp: ${data.expires_in}, refexp: ${data.refresh_expires_in})`
    : "not authenticated";
  requireElement<HTMLElement>("csrftoken").innerText = csrftoken;
}

function isLoggedIn(): void {
  fetch("/auth/status")
    .then((response) => response.json() as Promise<AuthStatus>)
    .then(setLoginStatus)
    .catch(() => {
      requireElement<HTMLElement>("loginStatus").innerHTML = "not authenticated";
    });
}

setInterval(isLoggedIn, 1000);

function queryString(params: Record<string, string>): string {
  return Object.entries(params)
    .map(([key, value]) => `${key}=${encodeURIComponent(value)}`)
    .join("&");
}

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

requireElement<HTMLButtonElement>("logout").onclick = () => {
  const oidCallbackUrl = `${window.location.origin}/auth/logoutcallback`;
  const appCallbackUrl = `${window.location.origin}/exampleapp/`;

  window.location.href = `/auth/logout?${queryString({
    redirect_uri: oidCallbackUrl,
    app_uri: appCallbackUrl,
  })}`;
};

requireElement<HTMLButtonElement>("refresh").onclick = () => {
  fetch("/auth/refresh", { method: "POST" })
    .then((response) => response.json())
    .then((refreshData) => console.log(refreshData));
};

requireElement<HTMLButtonElement>("requestcsrftoken").onclick = () => {
  fetch("/auth/csrftoken", { method: "POST" })
    .then((response) => response.json() as Promise<CsrfTokenResponse>)
    .then((data) => {
      csrftoken = data.token;
      requireElement<HTMLElement>("csrftoken").innerText = csrftoken;
    });
};

requireElement<HTMLButtonElement>("echorequest").onclick = () => {
  const headers = new Headers();
  headers.append("X-CSRF-TOKEN", csrftoken);
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
