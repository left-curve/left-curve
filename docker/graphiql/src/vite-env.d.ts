/// <reference types="vite/client" />

interface ImportMetaEnv {
    VITE_GRAPHQL_ENDPOINT: string;
}

interface ImportMeta {
    readonly env: ImportMetaEnv;
}

interface Window {
    GRAPHIQL_CONFIG?: {
        endpoint: string;
    };
}
