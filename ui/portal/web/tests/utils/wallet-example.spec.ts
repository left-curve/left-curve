import { test, expect } from '@playwright/test';
import { injectMockWallet } from './injectWallet';

test('mock wallet announces itself and handles requests', async ({ page }) => {
  // Inject the mock wallet before navigating
  await injectMockWallet(page);

  await page.goto('/');

  // Evaluate in the page context to verify the wallet is present
  const walletPresent = await page.evaluate(async () => {
    return new Promise<boolean>((resolve) => {
      // Check if already announced (if we were listening)
      // Or listen for the event
      const handler = (event: any) => {
        if (event.detail.info.name === 'Mock E2E Wallet') {
          window.removeEventListener('eip6963:announceProvider', handler);
          resolve(true);
        }
      };
      window.addEventListener('eip6963:announceProvider', handler);

      // Request announcement to trigger the event if we missed the initial one
      window.dispatchEvent(new Event('eip6963:requestProvider'));
    });
  });

  expect(walletPresent).toBe(true);

  // Verify provider methods
  const chainId = await page.evaluate(async () => {
    // @ts-ignore
    const provider = window.mockWalletProvider;
    return await provider.request({ method: 'eth_chainId' });
  });

  expect(chainId).toBe('0x1');
});
