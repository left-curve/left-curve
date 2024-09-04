import { ExampleAccountList, ExampleHeader } from "@leftcurve/react/components/examples";

function App() {
  return (
    <div className="flex flex-col min-h-screen w-full h-full bg-stone-200">
      <ExampleHeader />
      <ExampleAccountList />
    </div>
  );
}

export default App;
