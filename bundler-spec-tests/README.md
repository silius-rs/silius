## Bundler spec tests

Requirements:

Check instructions: https://github.com/eth-infinitism/bundler-spec-tests

Setup geth node and fund addresses:

```bash
docker compose up -d
geth attach http://127.0.0.1:8545
> eth.sendTransaction({ from: eth.accounts[0], to: "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266", value: 1000000000000000000 })
```

Deploy smart contracts (entrypoint):

```bash
git clone https://github.com/eth-infinitism/account-abstraction.git
cd account-abstraction
yarn install
yarn deploy --network localhost
```

Setup silius:

```bash
./target/release/silius node --wallet-path ./bundler-spec-tests/mnemonic-file.txt --network dev
```

Run tests:

```bash
git clone https://github.com/eth-infinitism/bundler-spec-tests.git
cd bundler-spec-tests
pdm install && pdm run update-deps
pdm test --entry-point 0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108 --url http://127.0.0.1:3000 --ethereum-node http://127.0.0.1:8545
pdm run pytest -rA -W ignore::DeprecationWarning -v tests/single/rpc/test_eth_supportedEntryPoints.py::test_eth_supportedEntryPoints[] --entry-point 0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108 --url http://127.0.0.1:3000 --ethereum-node http://127.0.0.1:8545
```
