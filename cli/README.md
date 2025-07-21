# CLI

Commands to execute logics in isolated context.

## Process tx events

```sh
$ cargo run -p cli -- index tx-process --digest=TX_DIGEST
```

## Checkpoint details

```sh
$ cargo run -p cli -- index checkpoint-details --checkpoint=NUMBER
```

## Lookup tx events

```sh
$ cargo run -p cli -- index tx-events --digest=TX_DIGEST
```
