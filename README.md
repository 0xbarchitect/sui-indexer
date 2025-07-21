# sui-mev
The MEV bot for SUI

## Prerequisites

- [Rust v1.85.0](https://www.rust-lang.org/tools/install)
- [Postgres v14](https://hub.docker.com/_/postgres)

## Setup

- Create `.env` file from template, fill in all necessary credentials and secrets

- Install `libpq`

>   - [Guide for MacOSX](./libpq_mac.md)

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

*Note: all diesel commands must be executed in the `db` directory*

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
