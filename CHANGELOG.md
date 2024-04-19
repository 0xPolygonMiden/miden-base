# Changelog

## 0.3.0 (TBD)

* [BREAKING] Removed the transaction script root output from the transaction kernel (#608).

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
* [BREAKING] Added account storage type parameter to `create_basic_wallet` and `create_basic_fungible_faucet` (miden-lib crate only) (#587).
* Removed serialization of source locations from account code (#590).

## 0.1.1 (2024-03-07) - `miden-objects` crate only

* Added `BlockHeader::mock()` method (#511)

## 0.1.0 (2024-03-05)

* Initial release.
