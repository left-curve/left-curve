
### Clone this repo and install dependencies

```shell
git clone https://github.com/left-curve/interface
cd interface
pnpm install
```

This repository uses `pnpm` for managing dependencies and running scripts. Below are the available scripts and their descriptions:

### Scripts

- **Run in dev mode for packages:**

  If you want to work on the packages, you can run the following command:

  ```sh
  pnpm dev:pkg
  ```

  This will start the development mode for all packages in the `./packages` directory.

- **Run in dev mode for the app:**

  If you want to work on the app, you can run the following command:

  ```sh
  pnpm dev:app
  ```

  This will start the development mode for the app located in the `./apps/superapp` directory.

### Build

- **Build all packages and the app:**

  To build all packages and the app, you can run:

  ```sh
  pnpm build:app
  ```

- **Build only the packages:**

  If you only want to build the packages, you can run:

  ```sh
  pnpm build:pkg
  ```

### Testing

- **Run tests:**

  If you want to run the tests on the packages, you can run the following command:

  ```sh
  pnpm test:pkg
  ```

### Linting

- **Lint the code:**

  If you want to lint on the packages, you can run the following command:

  ```sh
  pnpm lint:pkg
  ```

### Documentation

- **Generate documentation:**

  To generate the documentation, you can use:

  ```sh
  pnpm doc
  ```
