export const configuration = async () => {
  return Promise.resolve({
    baseUrl: process.env.IDENTITY_E2E_BASE_URL ?? "https://localhost:3000",
  });
};
