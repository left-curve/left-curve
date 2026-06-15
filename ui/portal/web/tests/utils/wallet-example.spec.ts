import { test, expect } from '@playwright/test';
import { DEFAULT_MOCK_WALLET_NAME } from '../mocks/eip6963';
import { injectMockWallet } from './injectWallet';

test('mock wallet announces itself and handles requests', async ({ page }) => {
  // Inject the mock wallet before navigating
  await injectMockWallet(page);

  await page.goto('/');

  // Evaluate in the page context to verify the wallet is present
  const walletPresent = await page.evaluate(async (walletName) => {
    return new Promise<boolean>((resolve) => {
      // Check if already announced (if we were listening)
      // Or listen for the event
      const handler = (event: Event) => {
        const walletEvent = event as CustomEvent<{ info: { name: string } }>;
        if (walletEvent.detail.info.name === walletName) {
          window.removeEventListener('eip6963:announceProvider', handler);
          resolve(true);
        }
      };
      window.addEventListener('eip6963:announceProvider', handler);

      // Request announcement to trigger the event if we missed the initial one
      window.dispatchEvent(new Event('eip6963:requestProvider'));
    });
  }, DEFAULT_MOCK_WALLET_NAME);

  expect(walletPresent).toBe(true);

  // Verify provider methods
  const chainId = await page.evaluate(async () => {
    // @ts-ignore
    const provider = window.mockWalletProvider;
    return await provider.request({ method: 'eth_chainId' });
  });

  expect(chainId).toBe('0x1');
});
