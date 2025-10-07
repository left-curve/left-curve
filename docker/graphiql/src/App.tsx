import React, {Suspense} from "react";

function App() {
    // noinspection JSUnusedLocalSymbols
    const GraphiQL = React.lazy(() => import("./GraphiQL.tsx"));
    const configuredEndpoint = window.GRAPHIQL_CONFIG?.endpoint;
    const inferredEndpoint = new URL(
        // Strip trailing slash so /graphql/ -> /graphql
        window.location.pathname.replace(/\/$/, ""),
        window.location.origin
    ).toString();
    const graphQlEndpoint = configuredEndpoint || inferredEndpoint;

    return (
        <Suspense fallback={<span className="loading">Loading…</span>}>
            <GraphiQL url={graphQlEndpoint}/>
        </Suspense>
    );
}

export default App;
