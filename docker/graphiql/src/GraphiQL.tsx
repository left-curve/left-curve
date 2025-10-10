import {explorerPlugin} from "@graphiql/plugin-explorer";
import {createGraphiQLFetcher} from "@graphiql/toolkit";
import {GraphiQL as Base} from "graphiql";

export function GraphiQL({url}: { url: string }) {
    const explorer = explorerPlugin();

    // derive wss/ws from http/https
    const subscriptionUrl = url.replace(/^http/i, "ws");

    const fetcher = createGraphiQLFetcher({
      url,
      subscriptionUrl,
    });

    return (
        <Base fetcher={fetcher} plugins={[explorer]}/>
    );
}

export default GraphiQL;
