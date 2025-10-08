import React, {Suspense} from "react";

function App() {
    // noinspection JSUnusedLocalSymbols
    const GraphiQL = React.lazy(() => import("./GraphiQL.tsx"));
    const graphQlEndpoint = window.GRAPHIQL_CONFIG?.endpoint ?? "http://localhost:4000/graphql";

    return (
        <Suspense fallback={<span className="loading">Loading…</span>}>
            <GraphiQL url={graphQlEndpoint}/>
        </Suspense>
    );
}

export default App;
