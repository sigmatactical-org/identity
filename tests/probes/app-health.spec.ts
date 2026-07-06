import { test, expect } from '@playwright/test';
import { configuration } from '../conf.ts';

test.describe('/app', () => {
  test.describe('/health', () => {
    test('returns JSON health report', async ({ request }) => {
      const conf = await configuration();
      const response = await request.get(conf.baseUrl + '/app/health');

      expect(response.status()).toBe(200);
      const body = await response.json();
      expect(body.service).toBe('identity');
      expect(body.status).toBe('healthy');
      expect(body.checks.database.status).toBe('healthy');
    });
  });
});
