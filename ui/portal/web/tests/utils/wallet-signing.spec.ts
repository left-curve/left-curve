import { test, expect } from '@playwright/test';
import { injectMockWallet } from './injectWallet';
import { verifyMessage } from 'viem';

test('mock wallet signs messages correctly', async ({ page }) => {
  const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
  const expectedAddress = '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266';

  // Inject the wallet with specific private key
  await injectMockWallet(page, { privateKey });

  await page.goto('/');

  // Execute signing in the browser
  const signature = await page.evaluate(async (address) => {
    // @ts-ignore
    const provider = window.mockWalletProvider;

    // Message to sign (hex encoded "hello world")
    // "hello world" -> 0x68656c6c6f20776f726c64
    const message = '0x68656c6c6f20776f726c64';

    return await provider.request({
      method: 'personal_sign',
      params: [message, address]
    });
  }, expectedAddress);

  // Verify signature locally
  const valid = await verifyMessage({
    address: expectedAddress,
    message: 'hello world', // The raw message content
    signature: signature as `0x${string}`,
  });

  expect(valid).toBe(true);
});
