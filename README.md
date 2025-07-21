# SUI indexer

The indexer for SUI onchain data. It listens to tx events in checkpoint, parse to structured data. The decoded data is saved to PostgreSQL for further processing. This data can be used to build analytics applications or trading bot.
Currently supports indexing data for protocols:

- [DEX] Cetus
- [DEX] Bluefin
- [DEX] Aftermath
- [DEX] Momentum
- [DEX] Turbos
- [DEX] FlowX
- [DEX] Bluemove
- [DEX] Obric
- [Lending] Navi
- [Lending] Suilend
- [Lending] Scallop
- [Oracle] Pyth

## Architecture

![system architecture](./docs/architecture.png)

## Prerequisites

- [Rust v1.85.0](https://www.rust-lang.org/tools/install)
- [Postgres v15](https://hub.docker.com/_/postgres)

## Setup

- Create `config.toml` file from template, fill in all necessary credentials and secrets

```sh
$ cp config.toml.example config.toml
```

- Install `libpq`

> - [Guide for MacOSX](./libpq_mac.md)

## Migration DB

- Install libpq (for Postgres driver)

```sh
$ sudo apt-get install libpq-dev
```

- Install [diesel CLI](https://diesel.rs/guides/getting-started) with Postgres support only.

```sh
$ cargo install diesel_cli --no-default-features --features postgres
```

- Export database connection URL

```sh
$ export DATABASE_URL=postgres://USERNAME:PASSWORD@HOST/DB
```

_Note: all diesel commands must be executed in the `db` directory_

- Create migration

```sh
$ diesel migration generate MIGRATION_NAME
```

- Apply migrations

```sh
$ diesel migration run
```

- Revert migrations

```sh
$ diesel migration revert
```

- List migrations

```sh
$ diesel migration list
```

- Rerun migrations (for testing)

```sh
$ diesel migration redo
```

## Compile

```sh
$ cargo build
```

## CLI tools

- Refer to [CLI docs](./cli/README.md) for information.

## Run server

```sh
$ cargo run -p server
```

## Troubleshoot

- Enable stack trace

```sh
$ RUST_BACKTRACE=1 cargo run -p server
```
