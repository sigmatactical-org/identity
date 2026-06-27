import { echoRequest } from "./echo-request";
import { isLoggedIn } from "./is-logged-in";
import { login } from "./login";
import { logout } from "./logout";
import { refresh } from "./refresh";
import { requestCsrfToken } from "./request-csrf-token";

login.attach();
logout.attach();
refresh.attach();
requestCsrfToken.attach();
echoRequest.attach();
isLoggedIn.startPolling();
