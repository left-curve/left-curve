export interface MockWalletOptions {
  accounts?: string[];
  chainId?: string;
  walletName?: string;
  walletUuid?: string;
  walletIcon?: string;
  walletRdns?: string;
}

export const getEip6963MockWalletScript = (options: MockWalletOptions = {}) => {
  const accounts = options.accounts || ["0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"];
  const chainId = options.chainId || "0x1"; // Mainnet
  const walletName = options.walletName || "Mock E2E Wallet";
  const walletUuid = options.walletUuid || "350670db-19fa-4704-a166-e52e178b59d2";
  const walletIcon =
    options.walletIcon ||
    "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZmlsbD0iIzM3N2RZmYiBkPSJNMCAwaDMydjMySDB6Ii8+PC9zdmc+";
  const walletRdns = options.walletRdns || "com.example.mockwallet";

  return `
    (() => {
      // Basic EventEmitter implementation
      class EventEmitter {
        constructor() {
          this.events = {};
        }

        on(event, listener) {
          if (!this.events[event]) this.events[event] = [];
          this.events[event].push(listener);
          return this;
        }

        removeListener(event, listener) {
          if (!this.events[event]) return this;
          this.events[event] = this.events[event].filter(l => l !== listener);
          return this;
        }

        emit(event, ...args) {
          if (!this.events[event]) return false;
          this.events[event].forEach(l => l(...args));
          return true;
        }
      }

      // Mock Provider Implementation
      class MockProvider extends EventEmitter {
        constructor(accounts, chainId) {
          super();
          this.isMetaMask = true; // Pretend to be MetaMask for broader compatibility
          this.isEIP6963 = true;
          this.accounts = accounts;
          this.chainId = chainId;
        }

        async request({ method, params }) {
          console.log('[MockProvider] request:', method, params);
          
          switch (method) {
            case 'eth_requestAccounts':
            case 'eth_accounts':
              return this.accounts;
              
            case 'eth_chainId':
              return this.chainId;
              
            case 'net_version':
              return parseInt(this.chainId, 16).toString();

            case 'wallet_switchEthereumChain':
              const chainIdParam = params?.[0]?.chainId;
              if (chainIdParam) {
                this.chainId = chainIdParam;
                this.emit('chainChanged', this.chainId);
                return null;
              }
              throw new Error('Invalid params for wallet_switchEthereumChain');
              
            case 'personal_sign':
            case 'eth_sign':
            case 'eth_signTypedData_v4':
              // Use exposed Playwright function if available
              if (window.mockWalletSign) {
                try {
                  return await window.mockWalletSign(method, params);
                } catch (error) {
                  console.error('[MockProvider] Signing failed:', error);
                  throw error;
                }
              }
              // Fallback to dummy signature
              return '0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef000000000000000000000000000000000000000000000000000000000000001b';

            default:
              // Log unknown methods
              console.warn('[MockProvider] Unhandled method:', method);
              // Return null for unhandled methods to avoid crashing, or implement specific logic as needed
              return null;
          }
        }
      }

      const mockProvider = new MockProvider(${JSON.stringify(accounts)}, '${chainId}');
      
      // Expose on window for test debugging/control
      window.mockWalletProvider = mockProvider;

      const info = {
        uuid: '${walletUuid}',
        name: '${walletName}',
        icon: '${walletIcon}',
        rdns: '${walletRdns}'
      };

      const announce = () => {
        const detail = Object.freeze({ info, provider: mockProvider });
        const event = new CustomEvent('eip6963:announceProvider', { detail });
        window.dispatchEvent(event);
        console.log('[MockWallet] Announced provider');
      };

      window.addEventListener('eip6963:requestProvider', () => {
        announce();
      });

      // Announce immediately
      announce();
    })();
  `;
};
