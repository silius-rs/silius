## Examples

### Creating your first user operation

```bash
cargo run --example user-operation
```

### Simple account - create 

```bash
SEED_PHRASE="test test test test test test test test test test test junk" BUNDLER_URL="http://127.0.0.1:3000" cargo run --example simple-account-create
```

### Simple account - deposit funds to entrypoint

```bash
SEED_PHRASE="test test test test test test test test test test test junk" PROVIDER_URL="http://127.0.0.1:3000" cargo run --example simple-account-deposit
```

### Simple account - transfer

```bash
SEED_PHRASE="test test test test test test test test test test test junk" BUNDLER_URL="http://127.0.0.1:3000" cargo run --example simple-account-transfer
```