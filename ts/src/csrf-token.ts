let value = "";

export const csrfToken = {
  get(): string {
    return value;
  },
  set(token: string): void {
    value = token;
  },
};
