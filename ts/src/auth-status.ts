export interface AuthStatus {
  authenticated: boolean;
  expires_in?: number;
  refresh_expires_in?: number;
}
