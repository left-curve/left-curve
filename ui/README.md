# UI

Web interface for accessing [Dango](../dango/)

## Packages

| Packages                       | Description                                                                                                                         |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| [`Applets Kit`](./applets/kit) | A common library for developing applets and communicate with dango portal                                                           |
| [`UI Config`](./config)        | Common configurations for ui elements such as tailwind, fonts, etc...                                                               |
| [`Core Store`](./store/core)   | It allows connect with dango blockchain, connect multiples wallets, manages accounts, and enables interaction with smart contracts. |
| [`React Store`](./store/react) | It wraps store and all their actions into react-hooks and wrap the state in a react provider with hydration for ssr                 |
| [`Proxy`](./workers/proxy)     | Cloudflare worker used as proxy for devnet rpc                                                                                      |

----

## Apps

| Apps                                    | Description                                       |
| --------------------------------------- | ------------------------------------------------- |
| [`Portal Website`](./ui/portal/website) | Dango portal website                              |
| [`Portal App`](./ui/portal/app)         | Dango portal react native app for Android and iOS |
| [`Website`](./packages/website/)        | Dango landing page                                |

----

## Applets

| Applet | Description |
| ------ | ----------- |
