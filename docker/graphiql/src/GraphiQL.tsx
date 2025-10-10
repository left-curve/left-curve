import {explorerPlugin} from "@graphiql/plugin-explorer";
import {createGraphiQLFetcher} from "@graphiql/toolkit";
import {GraphiQL as Base} from "graphiql";

export function GraphiQL({url}: { url: string }) {
    const subscriptionUrl = url.replace(/^https?:/, "ws:");

    const fetcher = createGraphiQLFetcher({
      url,
      subscriptionUrl
    });

    return <Base fetcher={fetcher} plugins={[explorerPlugin()]} />;
}

export default GraphiQL;
