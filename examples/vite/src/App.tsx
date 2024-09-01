import { useChainId, useConnectors } from "@leftcurve/react";

function App() {
  const chainId = useChainId();
  const connectors = useConnectors();

  console.log(chainId, connectors);
  return (
    <div className="flex items-center justify-center min-h-screen w-full h-full bg-stone-900">
      <h1 className="text-3xl font-bold text-neutral-100">Hello world!</h1>
    </div>
  );
}

export default App;
