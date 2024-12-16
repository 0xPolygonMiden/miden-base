## What has changed

`AccountId` previously fit into a felt or ~64 bits. Now its representation is two felts:

```rust
pub struct AccountId {
    first_felt: Felt,
    second_felt: Felt,
}
```

In Rust, the first and second felts are always called as such, typically accessed with `id.first_felt()` and `id.second_felt()`.
In MASM, the first felt is the `hi` felt and the second felt is the `lo` felt and its typical stack representation is [`account_id_hi, account_id_lo`].

The layout of an ID is:

```text
1st felt: [zero bit | random (55 bits) | storage mode (2 bits) | type (2 bits) | version (4 bits)]
2nd felt: [block_epoch (16 bits) | random (40 bits) | 8 zero bits]
```

See the `AccountId` documentation for details on the layout.

There is a type `AccountIdPrefix` representing the validated first felt of an `AccountId` which is primarily used for non-fungible assets and for asset deserialization. Ideally we would make that private, but for now it must be public for the constructors of non fungible assets.

### Notable Changes

Here is a quick overview of what changes with this PR. Layouts are in "memory order", that is, stack order would be the reverse of that.

- Fungible Asset representation:
  - previously: `[amount, 0, 0, faucet_id]`
  - now:        `[amount, 0, faucet_id_lo, faucet_id_hi]`
- Non-Fungible Asset representation:
  - previously: `[hash_0, faucet_id, hash_2, hash_3]`
  - now:        `[hash_0, faucet_id_hi, hash_2, hash_3]`
- NoteMetadata
  - previously: `[tag, sender_acct_id, encoded_type_and_ex_hint, aux]`
  - now:        `[sender_hi, sender_lo_type_and_hint_tag, note_tag_hint_payload, aux]`
  - The NoteMetadata `AfterBlock` can only take values less than `u32::MAX` so that when encoded together with the note tag in a felt (i.e. `note_tag_hint_payload` above) felt validity is guaranteed.
- The serialization of `AccountIdPrefix` is compatible with `AccountId` to allow for deserialization of a prefix from a serialized `AccountId`. This is used for the next point.
- `AccountIdPrefix` serialization is such that the first byte of the serialized layout contains the metadata. This is used in the asset deserialization to read the type and based on that deserialize the bytes as a fungible or non-fungible asset.
- The `NoteTag::from_account_id` effectively takes the most significant bits of `id_first_felt << 1`. This means the high zero bit is ignored with the rationale being that it would not add any value for matching an account ID against a note tag, since all ID's high bits are zero. Let me know if this doesn't make sense.
- The previous account creation tests were replaced because they now require a `MockChain` due to the dependence of the account ID on an anchor block.
- The layout of account IDs in various keys, stacks and hashes across Rust and Masm is not entirely consistent I think because it's not always quite clear to me whether a layout needs to be reversed or not. What is also unclear to me is in what order IDs should be layed out within layouts like `[account_nonce, 0, account_id_hi, account_id_lo]`. So anything related to that would be particularly helpful to have reviewed.
- I snuck in some small improvements to the debugging experience but I'd like to revisit this topic in a future PR.

This PR does not change anything about the account tree, e.g. the `accounts: SimpleSmt<ACCOUNT_TREE_DEPTH>` field in `MockChain`.

I think a rough sensible review order would be AccountId + AccountIdPrefix followed by the changes to assets, all in Rust.
Then moving over to MASM.

## Open Questions

- Should we enforce MIN_ACCOUNT_ONES in both first and second felt? For now only the first felt is enforced with the only rationale being that the first felt is used as the key in the asset SMT for fungible assets. But if we treat this as an implementation detail then it might make sense to also enforce something similar on the second felt?
- Do we need a serialization format for `NoteMetadata` that is different from the `Word` encoding? We had a different one before this PR, but now the serialization format and word encoding are identical.

## Follow-Up Tasks

There are still some TODOs in the code and those could mostly be addressed in a follow-up PR so this PR could would not necessarily be blocked by that (checked boxes are included in this PR).

- [ ] Introduce `AccountIdError`. There are now new error conditions for IDs and a separate error might be cleaner.
  - With the introduction of this error it might be possible to make `AccountId::new_dummy` a `const fn` and then we could possibly replace the somewhat duplicate functionality of `miden_objects::testing::account_id::account_id`.
- [ ] Quite a bit of documentation (mostly Rust) needs to be updated now and that hasn't been done yet for priority reasons.
- [ ] Implement Display for `AccountType` and use it in error messages.
- [ ] Remove unnecessary code in `generate_account_seed`.
- [ ] Remove `build.rs` constants patching for POW which we no longer have.
- [ ] Move type, storage mode, version out into their own modules to reduce size of `account_id.rs`.
- [ ] Move newly introduced duplicate procedures into the new util library (after #1002).
- [ ] Go through error messages and double check if they are up to date (e.g. ones previously mentioning "account id" should now likely mention first or second felt for accuracy).
- [ ] `validate_fungible_asset_origin` validates the account ID and then calls `validate_fungible_asset` which validates it again check if we can remove some redundancy here.
- [x] Deal with `NoteExecutionHint::AfterBlock` being constructable by users but it now cannot contain u32::MAX and we might want to prevent that.
- [x] Update and test `build_swap_tag`.
- [ ] Add lots of new tests for changed things: version and epoch in ID, test new and changed MASM procedures more thoroughly.
- [ ] Think about account ID encoding for network identifier and error detection.

## Why it changed

See #140, although I'd like to ideally add a summary here.

closes #140

## Misc
- account_id_hi is the first and account_id_lo is the second felt. Makes somewhat sense because in hex the first felt has the more significant bits and in the u128 repr as well.

TODO:
- ~~Reorder version in account id~~
- ~~Swap account id order in fungible asset~~
- ~~Should AccountIdPrefix and AccountId have a compatible layout for hex, byte and serialized reprs? If so, add test for this.~~
- ~~Test Account Id creation/validation with reference block = epoch block.~~
- Test validate_id in masm.
- Test Account ID invalid version, epoch.
- Rename faucet_id to faucet_id_prefix
- Double check modified error's messages
- Use swap.N in is_id_eq
- Add Where clause on masm docs
- Remove test_ prefix from tests (in account_id, for example)

- Make sure all `SERIALIZED_SIZE` constants are tested and possibly consider adding them consistently for all types that need `get_size_hint`.

## Why it changed

<hr>

Hashes updated:
- Account Hash
- TX Hash in block (`compute_tx_hash`)
- Rpo Falcon 512 message hash (see masm file)

## VM Bugs
- debug.stack.0 panics 
thread 'tests::kernel_tests::test_prologue::test_transaction_prologue' panicked at /home/philipp/.cargo/registry/src/index.crates.io-6f17d22bba15001f/miden-processor-0.11.0/src/host/debug.rs:59:47:
attempt to subtract with overflow
- debug/trace at end of loop behaves differently as when put elsewhere

Fungible Assets
memory order: [300, 0, id_hi, id_lo]
stack order: [id_lo, id_hi, 0, 300]

should become:
memory order: [300, 0, id_lo, id_hi]
stack order: [id_hi, id_lo, 0, 300]

## VM Debugging

```
...
clk 3094: op `dropw` in 'miden::note::get_sender'
clk 3095: op `dropw` in 'miden::note::get_sender'
clk 3096: op `movdn.3` in 'miden::note::get_sender'
clk 3097: op `movdn.3` in 'miden::note::get_sender'
clk 3098: op `drop` in 'miden::note::get_sender'
clk 3099: op `drop` in 'miden::note::get_sender'
clk 3103: op `movup.2` in 'miden::account::is_id_eq'
clk 3104: op `eq` in 'miden::account::is_id_eq'
clk 3105: op `swap` in 'miden::account::is_id_eq'
clk 3106: op `movup.2` in 'miden::account::is_id_eq'
clk 3107: op `eq` in 'miden::account::is_id_eq'
clk 3108: op `and` in 'miden::account::is_id_eq'
execution error: Assertion failed at clock cycle 3112 with error code 131155: P2IDR's reclaimer is not the original sender
```
## Order

- Should memory order of account ids be:
1) [account_id_lo, account_id_hi] <- this option is consistent with how words are layed out
2) [account_id_hi, account_id_lo]
Answer: 1), meaning stack order would be [hi, lo]