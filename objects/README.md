# Miden Objects

This crates contains core componenet definitions to be used by Miden clients
and the rollup.

# Modules

Here is a broad overview of each module, with links to additional documentation.


## Accounts

Structures used to define accounts. Including abstractions over its code,
storage, and vault.

[Documentation](https://0xpolygonmiden.github.io/miden-base/architecture/accounts.html).

## Assets

Structures used to define funginble and non-fungible assets. Accounts own
assets and store them in their vaults.

[Documentation](https://0xpolygonmiden.github.io/miden-base/architecture/assets.html)


## Block

Structures used to define a block. These objects contains authentication
structures, merkle trees, used to represent the state of the rollup at a given
point in time.

## Notes

Structures used to define notes. Notes are messages that contain code and
assets. They describe their own behaviour and allow for interation among
accounts. Notes come in multiple flavors, refer to the docs for additional
details.

[Documentation](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html)

## Transaction

Structures used to define transactions. Transactions represent changes to an
account, and possibly the consumption of notes. The objects in this module
allow for the representation of transactions at multiple stages of its
lifecycle, from creation, to data aggregation, execution with trace collection,
and finally an executed transaction with a corresponding STARK proof.

[Documentation](https://0xpolygonmiden.github.io/miden-base/architecture/transactions.html).

# Features

Description of this crate's feature:

| Features     | Description                                                                                   |
| ------------ | --------------------------------------------------------------------------------------------- |
| `std`        | Enable usage of Rust's `std`, use `--no-default-features` for `no-std` support.               |
| `concurrent` | Enables concurrent code to speed up runtime execution.                                        |
| `serde`      | Enables serialization of most objects via `serde`.                                            |
| `testing`    | Enables testing utilities and reduces proof-of-work requirements to speed up tests' runtimes. |

## License

This project is [MIT licensed](../LICENSE).
