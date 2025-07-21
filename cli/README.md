# CLI

Commands to execute MEV bot logics in isolated context.

## DB commands

### Import lending markets

```sh
$ cargo run -p cli -- db import-markets
```

### Import coins info

```sh
$ cargo run -p cli -- db import-coins
```

## Aggregate borrowers from borrows and deposits

```sh
$ cargo run -p cli -- db aggregate-borrowers
```

## Export `ready` borrowers to file

```sh
$ cargo run -p cli -- db export-borrowers --file-path=FILENAME
```

## Import borrowers from file

```sh
$ cargo run -p cli -- db import-borrowers --file-path=FILENAME
```

## Index commands

### Tx event

```sh
$ cargo run -p cli -- index tx-details --digest=TX_DIGEST
```

### Shio event

- Copy and paste the auction json content to `shio.json`

```sh
$ cargo run -p cli -- index shio-event
```

### Hermes event

- Copy and paste the message json content to `hermes.json`

```sh
$ cargo run -p cli -- index hermes-event
```

### Fetch borrower portfolios

```sh
$ cargo run -p cli -- index fetch-borrower-portfolios
```

### Lookup borrower portfolio onchain

```sh
$ cargo run -p cli -- index lookup-portfolio --platform=PLATFORM --borrower=ADDRESS
```

## Alpha commands

### Find arbitrage paths

```sh
$ cargo run -p cli -- alpha find-arb-paths
```

### Find borrowers

```sh
$ cargo run -p cli -- alpha find-borrowers
```

### Calculate borrower HF

```sh
$ cargo run -p cli -- alpha calc-borrower-hf --platform=PLATFORM --address=BORROWER
```

### Calculate amount repay for liquidation

```sh
$ cargo run -p cli -- alpha calc-amount-repay --platform=PLATFORM --address=BORROWER
```

### Calculate borrower debt value in USD

```sh
$ cargo run -p cli -- alpha calc-borrower-debt --platform=PLATFORM --address=BORROWER
```

## Execution commands

### Generate bot wallets

```sh
$ cargo run -p cli -- execute generate-wallets --count=NUMBER
```

### Fund bots

```sh
$ cargo run -p cli -- execute fund
```

### Harvest bots

```sh
$ cargo run -p cli -- execute harvest
```

### Update price feeds

- Update prices using VAA from API

```sh
$ cargo run -p cli -- execute update-price-feeds --coins=COMMA_SEPARATED_COIN_TYPES
```

- Update prices using VAA from Websocket

```sh
$ cargo run -p cli -- execute update-price-feeds --coins=SINGLE_COIN_TYPE --use-wss
```

- Update prices using VAA and submit tx via Shio endpoint

```sh
$ cargo run -p cli -- execute update-price-feeds --coins=COMMA_SEPARATED_COIN_TYPES --use-shio-endpoint
```
