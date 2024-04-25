# Miden transactions benchmark

This crate contains an executable used for benchmarking transactions. 

For each transaction, data is collected on the number of cycles required to complete:
- Prologue
- All notes processing
- Each note execution
- Transaction script processing
- Epilogue

## Usage

To run the benchmark you can use [cargo-make](https://github.com/sagiegurari/cargo-make) with the following command present in our [Makefile.toml](Makefile.toml):

```shell
cargo make bench-tx
```

Results of the benchmark are stored in the [bench-tx.json](bench-tx.json) file.

## License

This project is [MIT licensed](../LICENSE).
