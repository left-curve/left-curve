import { useAccount, useChainId, useConnectors } from "@leftcurve/react";
import { Button, Modal } from "@leftcurve/react/components";
import React, { useEffect } from "react";

function App() {
  const [showModal, setShowModal] = React.useState(false);
  const { isConnected, username, connector } = useAccount();
  const connectors = useConnectors();
  const chainId = useChainId();

  useEffect(() => {
    if (isConnected) {
      setShowModal(false);
    }
  }, [isConnected]);

  return (
    <div className="flex min-h-screen w-full h-full bg-stone-200">
      <header className="flex h-16 w-full items-center justify-between px-4 md:px-6 bg-stone-100">
        <div className="flex items-center gap-2">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            className="w-6 h-6 text-primary-500"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m8 3 4 8 5-5 5 15H2L8 3z" />
          </svg>
          <Modal showModal={showModal} onClose={() => setShowModal(false)}>
            <div className="flex flex-col px-4 py-8 text-neutral-100 rounded-3xl bg-neutral-700 min-h-[350px] min-w-[300px]">
              <p className="text-2xl text-center font-bold">Connect Wallet</p>
              <ul className="flex flex-col px-2 py-4">
                {connectors.map((connector) => {
                  return (
                    <Button
                      key={connector.name}
                      className="bg-neutral-600 hover:bg-neutral-500 py-6"
                      onClick={() => connector.connect({ username: "test", chainId })}
                    >
                      <span className="flex w-full items-center justify-between">
                        <span className="text-lg">{connector.name}</span>
                        <img
                          src={connector.icon}
                          alt={connector.name}
                          className="w-10 h-10 text-white"
                        />
                      </span>
                    </Button>
                  );
                })}
              </ul>
            </div>
          </Modal>
          <span className="text-lg font-semibold">Example Vite</span>
        </div>
        <Button
          variant="danger"
          className="border border-slate-500 relative min-w-28 group"
          onClick={() => (isConnected ? connector?.disconnect() : setShowModal(true))}
        >
          {!isConnected ? <p>Connect</p> : null}
          {isConnected ? (
            <p className="text-center">
              <span className="block group-hover:hidden">{username}</span>
              <span className="hidden group-hover:block">Disconnect</span>
            </p>
          ) : null}
        </Button>
      </header>
    </div>
  );
}

export default App;
