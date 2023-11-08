## Examples

### Creating your first user operation

```bash
cd user-operation
cargo run --example user_operation
```

### Simple account - create 

```bash
cd simple-account
SEED_PHRASE="test test test test test test test test test test test junk" BUNDLER_URL="http://127.0.0.1:3000" cargo run --example create
```

### Simple account - deposit funds to entrypoint

```bash
cd simple-account
SEED_PHRASE="test test test test test test test test test test test junk" PROVIDER_URL="http://127.0.0.1:3000" cargo run --example deposit
```

### Simple account - transfer

```bash
cd simple-account
SEED_PHRASE="test test test test test test test test test test test junk" BUNDLER_URL="http://127.0.0.1:3000" cargo run --example transfer
```

### Storage - memory

```bash
cd storage
PROVIDER_URL="http://127.0.0.1:8545" cargo run --example memory
```

### Storage - database

```bash
cd storage
PROVIDER_URL="http://127.0.0.1:8545" cargo run --example database
```
