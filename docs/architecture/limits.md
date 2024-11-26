# Limits

The following are the current limits enforced in the `miden-base` and `miden-node`:

## Accounts
- Max assets per account: **no limit**.
- Max top-level storage slots per account: **255**. Each storage slot can contain an unlimited 
  amount of data (e.g., if the storage slot contains an array or a map).
- Max code size per account: **no limit** (but we plan to enforce a code size limit in the future, 
  at least for public accounts).

## Notes
- Min assets per note: **0**.
- Max assets per note: **255**.
- Max inputs per note: **128**. The value can be represented as a single byte while being 
  evenly divisible by 8.
- Max code size per note: **no limit** (but we plan to enforce a code size limit in the future,
  at least for public notes).

## Transactions
- Max input notes per transaction: **1024**.
- Max output notes per transaction: **1024**.
- Max code size of tx script: **no limit** (but we plan to enforce a code size limit in the future).
- Max number of VM cycles: **$2^{30}$**.

## Batches
- Max number of input notes: **1024**.
- Max number of output notes: **1024**.
- Max number of accounts: **1024**.
- Max number of VM cycles: **$2^{30}$**.

## Blocks
- Max batches per block: **64**.
- Max number of accounts: **65536** (*max accounts per batch × max number of batches*).
- Max number of input notes: **65536** (*max notes per batch × max number of batches*).
- Max number of output notes: **65536** (*max notes per batch × max number of batches*).
- Max public data size (for both notes and accounts): **no limit**.
