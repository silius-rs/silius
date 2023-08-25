## Examples

```bash
cargo run --example user-operation
```

### Create simple account


```bash
SEED_PHRASE="test test test test test test test test test test test junk" BUNDLER_URL="http://127.0.0.1:3000" cargo run --example simple_account_create
```

### Deposit funds to entrypoint

```bash
SEED_PHRASE="test test test test test test test test test test test junk" PROVIDER_URL="http://127.0.0.1:3000" cargo run --example deposit
```


### Simple Account Transfer

```bash
SEED_PHRASE="test test test test test test test test test test test junk" BUNDLER_URL="http://127.0.0.1:3000" cargo run --example transfer
```