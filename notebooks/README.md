# Notebooks

Python prototypes of algorithms used in [Dango](https://dango.exchange/) DeFi products.

## How to use

We recommend [uv](https://github.com/astral-sh/uv) for managing Python development environments. To install on macOS:

```zsh
brew install uv
```

Create a virtual environment. **In the root directory of this repository**,

```zsh
uv venv
```

Activate the virtual environment:

```zsh
source .venv/bin/activate
```

Install the dependencies:

```zsh
uv sync
```

Run JupyterLab:

```zsh
jupyter lab --notebook-dir=notebooks/
```
