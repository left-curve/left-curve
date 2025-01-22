# UI

Web interface for accessing [Dango](../dango/)

## Packages

| Packages                       | Description                                                                                                                         |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| [`applets-kit`](./applets/kit) | A common library for developing applets and communicate with dango portal                                                           |
| [`ui-config`](./config)        | Common configurations for ui elements such as tailwind, fonts, etc...                                                               |
| [`store`](./store/core)        | It allows connect with dango blockchain, connect multiples wallets, manages accounts, and enables interaction with smart contracts. |
| [`store-react`](./store/react) | It wraps store and all their actions into react-hooks and wrap the state in a react provider with hydration for ssr                 |
| [`proxy`](./workers/proxy)     | Cloudflare worker used as proxy for devnet rpc                                                                                      |

----

## Apps

| Apps                                    | Description                                       |
| --------------------------------------- | ------------------------------------------------- |
| [`portal-website`](./ui/portal/website) | Dango portal website                              |
| [`portal-app`](./ui/portal/app)         | Dango portal react native app for Android and iOS |
| [`website`](./website/)                 | Dango landing page                                |

----

## Applets

| Applet | Description |
| ------ | ----------- |
