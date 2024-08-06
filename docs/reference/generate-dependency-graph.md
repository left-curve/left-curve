# Generate dependency graph

Dependency relations of the crates in this repository are described by the following [Graphviz](https://graphviz.org/) code:

```graphviz
digraph G {
  node [fontname="Helvetica" style=filled fillcolor=yellow];

  account -> ffi;
  account -> storage;
  account -> types;

  bank -> ffi;
  bank -> storage;
  bank -> types;

  taxman -> bank;
  taxman -> ffi;
  taxman -> storage;
  taxman -> types;

  testing -> app;
  testing -> account;
  testing -> bank;
  testing -> crypto;
  testing -> "db/memory";
  testing -> taxman;
  testing -> types;
  testing -> "vm/rust";

  app -> storage;
  app -> types;

  client -> account;
  client -> crypto;
  client -> types;

  "db/disk" -> app;
  "db/disk" -> jmt;
  "db/disk" -> types;

  "db/memory" -> app;
  "db/memory" -> jmt;
  "db/memory" -> types;

  ffi -> types;

  jmt -> storage;
  jmt -> types;

  std -> ffi;
  std -> macros;
  std -> storage;
  std -> testing;
  std -> types;

  storage -> types;

  "vm/rust" -> app;
  "vm/rust" -> crypto;
  "vm/rust" -> types;

  "vm/wasm" -> app;
  "vm/wasm" -> crypto;
  "vm/wasm" -> types;
}
```

Install [Graphviz CLI](https://formulae.brew.sh/formula/graphviz) on macOS:

```bash
brew install graphviz
```

Generate SVG from a file:

```bash
dot -Tsvg input.dot
```

Generate SVG from stdin:

```bash
echo 'digraph { a -> b }' | dot -Tsvg > output.svg
```

Alternatively, use the [online visual editor](http://magjac.com/graphviz-visual-editor/).
