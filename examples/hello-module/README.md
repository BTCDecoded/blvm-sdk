# Hello Module Example

Minimal BLVM module demonstrating all extension points from the plan.

## Layout

When deployed to a blvm assembly:

```
modules/hello/
├── module.toml
├── config.toml
└── hello-module          # binary from cargo build --example hello-module
```

## Extension Points Demonstrated

- `#[module_cli(name = "hello")]` — CLI command name
- `#[cli_subcommand]` — subcommand handlers (greet)
- `#[module_config(name = "hello")]` — config struct
- `#[blvm_module]` — module struct (placeholder)

## Usage

1. Build: `cargo build --example hello-module --release`
2. Copy binary to `modules/hello/hello-module`
3. Add `module.toml` and `config.toml`
4. Load via `blvm module load hello` or enable in config
5. Run: `blvm hello greet` or `blvm hello greet -- Alice`

## module.toml

```toml
name = "hello"
version = "0.1"
entry_point = "hello-module"
```

## config.toml

```toml
greeting = "Hello"
```
