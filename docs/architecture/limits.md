# Limits

The following are the current limits enforced in the `miden-base` and `miden-node`:

## Accounts
- Max assets per account: **no limit**.
- Max top-level storage slots per account: **255**. Each storage slot can contain an unlimited 
  amount of data (e.g., if the storage slot contains an array or a map).
- Max code size per account: **no limit** (but we plan to enforce code size limit in the future, 
  at least for public accounts).

## Notes
- Min assets per note: **1**.
- Max assets per note: **256**.
- Max inputs per note: **128**. The value can be represented using as a single byte while being 
  evenly divisible by 8.
- Max code size per note: **no limit** (but we plan to enforce code size limit in the future,
  at least for public notes).

## Transactions
- Max input notes per transaction: **1024**.
- Max output notes per transaction: **1024**.
- Max code size of tx script: **no limit** (but we plan to enforce code size limit in the future,
  at least for public notes).

## VM Cycles
- Max number of VM cycles: technically **no limit**, but practically a single transaction cannot take 
  more than $2^{29}$ cycles. Even more practically, anything above $2^{20}$ cycles will require 10GB+ 
  of RAM (e.g., $2^{24}$ cycles will require ~256GB of RAM, though, with optimizations, we can bring 
  it down to ~128GB).

## Batches
- Max number of input notes: **1024**.
- Max number of output notes: **1024**.
- Max number of transactions: **1024**.

## Blocks
- Max batches per block: **64**.
- Max number of transactions: **65536** (*max transactions per batch × max number of batches*).
- Max number of input notes: **65536** (*max notes per batch × max number of batches*).
- Max number of output notes: **65536** (*max notes per batch × max number of batches*).

