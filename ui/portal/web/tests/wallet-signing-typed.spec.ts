import { test, expect } from '@playwright/test';
import { injectMockWallet } from './utils/injectWallet';
import { verifyTypedData } from 'viem';

test('mock wallet signs typed data (EIP-712) correctly', async ({ page }) => {
  const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
  const expectedAddress = '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266';

  // Inject the wallet
  await injectMockWallet(page, { privateKey });

  await page.goto('/');

  // Example EIP-712 Typed Data
  const domain = {
    name: 'Ether Mail',
    version: '1',
    chainId: 1,
    verifyingContract: '0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC',
  } as const;

  const types = {
    Person: [
      { name: 'name', type: 'string' },
      { name: 'wallet', type: 'address' },
    ],
    Mail: [
      { name: 'from', type: 'Person' },
      { name: 'to', type: 'Person' },
      { name: 'contents', type: 'string' },
    ],
  } as const;

  const message = {
    from: {
      name: 'Cow',
      wallet: '0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826',
    },
    to: {
      name: 'Bob',
      wallet: '0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB',
    },
    contents: 'Hello, Bob!',
  } as const;

  // Execute signing in the browser
  const signature = await page.evaluate(async ({ address, domain, types, message }) => {
    // @ts-ignore
    const provider = window.mockWalletProvider;

    const typedData = JSON.stringify({
      domain,
      types,
      primaryType: 'Mail',
      message
    });

    return await provider.request({
      method: 'eth_signTypedData_v4',
      params: [address, typedData]
    });
  }, {
    address: expectedAddress,
    domain,
    types,
    message
  });

  // Verify signature locally
  const valid = await verifyTypedData({
    address: expectedAddress,
    domain,
    types,
    primaryType: 'Mail',
    message,
    signature: signature as `0x${string}`,
  });

  expect(valid).toBe(true);
});
