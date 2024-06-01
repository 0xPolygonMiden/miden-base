# Changelog

## 0.4.0 (TBD)

### Enhancements

* [BREAKING] Create `auth` module for `TransactionAuthenticator` and other related objects (#714).
* Implemented `build_recipient_hash` to build recipient hash for custom notes (#710)
* Replaced `cargo-make` with just `make` for running tasks (#696).
* [BREAKING] Introduce `OutputNote::Partial` variant (#698).
* [BREAKING] Split `Account` struct constructor into `new()` and `from_parts()` (#699).
* [BREAKING] Changed the encoding of inputs notes in the advice map for consumed notes. Now the data
  is prefixed by its length, and the input and output notes encoding match (#707).
* Added validation for the output stack to make sure it was properly cleaned (#717).

## 0.3.1 (2024-06-12)
* Replaced `cargo-make` with just `make` for running tasks (#696).
* Made `DataStore` conditionally async using `winter-maybe-async` (#725)
* Fixed `StorageMap`s implementation and included into apply_delta (#745)

## 0.3.0 (2024-05-14)

* Introduce the `miden-bench-tx` crate used for transactions benchmarking (#577).
* [BREAKING] Removed the transaction script root output from the transaction kernel (#608).
* [BREAKING] Refactored account update details, moved `Block` to `miden-objects` (#618, #621).
* [BREAKING] Made `TransactionExecutor` generic over `TransactionAuthenticator` (#628).
* [BREAKING] Changed type of `version` and `timestamp` fields to `u32`, moved `version` to the beginning of block header (#639).
* [BREAKING] Renamed `NoteEnvelope` into `NoteHeader` and introduced `NoteDetails` (#664).
* [BREAKING] Updated `create_swap_note()` procedure to return `NoteDetails` and defined SWAP note tag format (#665).
* Implemented `OutputNoteBuilder` (#669).
* [BREAKING] Added support for full details of private notes, renamed `OutputNote` variants and changed their meaning (#673).
* [BREAKING] Added `add_asset_to_note` procedure to the transaction kernel (#674).
* Made `TransactionArgs::add_expected_output_note()` more flexible (#681).
* [BREAKING] Enabled support for notes without assets and refactored `create_note` procedure in the transaction kernel (#686).

## 0.2.3 (2024-04-26) - `miden-tx` crate only

* Fixed handling of debug mode in `TransactionExecutor` (#627)

## 0.2.2 (2024-04-23) - `miden-tx` crate only

* Added `with_debug_mode()` methods to `TransactionCompiler` and `TransactionExecutor` (#562).

## 0.2.1 (2024-04-12)

* [BREAKING] Return a reference to `NoteMetadata` from output notes (#593).
* Add more type conversions for `NoteType` (#597).
* Fix note input padding for expected output notes (#598).

## 0.2.0 (2024-04-11)

* [BREAKING] Implement support for public accounts (#481, #485, #538).
* [BREAKING] Implement support for public notes (#515, #540, #572).
* Improved `ProvenTransaction` validation (#532).
* [BREAKING] Updated `no-std` setup (#533).
* Improved `ProvenTransaction` serialization (#543).
* Implemented note tree wrapper structs (#560).
* [BREAKING] Migrated to v0.9 version of Miden VM (#567).
* [BREAKING] Added account storage type parameter to `create_basic_wallet` and `create_basic_fungible_faucet` (miden-lib
  crate only) (#587).
* Removed serialization of source locations from account code (#590).

## 0.1.1 (2024-03-07) - `miden-objects` crate only

* Added `BlockHeader::mock()` method (#511)

## 0.1.0 (2024-03-05)

* Initial release.
