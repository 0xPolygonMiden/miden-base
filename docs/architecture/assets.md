# Asset

> Fungible, Non-fungible, Native, and Non-native assets in the Miden protocol.

## What is the purpose of an asset?

In Miden, assets serve as the primary means of expressing and transferring value between [accounts](accounts.md) through [notes](notes.md). They are designed with four key principles in mind:

1. **Parallelizable exchange:**  
    By managing ownership and transfers directly at the account level instead of relying on global structures like ERC20 contracts, accounts can exchange assets simultaneously, boosting scalability and efficiency.

2. **Self-Sovereign Ownership:**  
   Assets are stored in the accounts directly. This ensures that users retain complete control over their assets.

3. **Censorship resistance:**  
   Users can transact freely and anonymously with no single contract or entity controlling asset transfers. This reduces the risk of censored transactions, resulting in a more open and resilient system.

4. **Flexible fee payment:**  
   Unlike protocols that require a specific base asset for fees, Miden allows users to pay fees in any supported asset. This flexibility simplifies the user experience.

## What is an asset?

An asset in Miden is a unit of value that can be transferred from one [account](accounts.md) to another using [notes](notes.md).

## Native asset

> All data structures following the Miden asset model that can be exchanged.

Native assets adhere to the Miden asset model (encoding, issuance, storage). Every native asset is encoded using a single `Word` (4 field elements). This `Word` includes both the [ID](accounts.md#id) of the issuing account and the asset details.

### Issuance

> **Info**
> - Only [faucet](accounts.md#account-type) accounts can issue assets.

Faucets can issue either fungible or non-fungible assets as defined at account creation. The faucet's code specifies the asset minting conditions: i.e., how, when, and by whom these assets can be minted. Once minted, they can be transferred to other accounts using notes.

![Architecture core concepts](../img/architecture/asset/asset-issuance.png)

### Type

#### Fungible asset

Fungible assets are encoded with the amount and the `faucet_id` of the issuing faucet. The amount is always `$2^{63} - 1$` or smaller, representing the maximum supply for any fungible asset. Examples include ETH and various stablecoins (e.g., DAI, USDT, USDC).

If the `faucet_id` of ETH is `2`, 100 ETH is encoded as:  
`[100, 0, 0, 2]`

The zeros in the middle positions distinguish fungible from non-fungible assets.

#### Non-fungible asset

Non-fungible assets are encoded by hashing the asset data into a `Word` and placing the `faucet_id` as the second element. Examples include NFTs like a DevCon ticket.

A non-fungible asset is encoded as:  
`[e0, faucet_id, e2, e3]`

The `faucet_id` at position `1` distinguishes non-fungible from fungible assets.

### Storage

[Accounts](accounts.md) and [notes](notes.md) have vaults used for asset storage.

![Architecture core concepts](../img/architecture/asset/asset-storage.png)

### Burning

Assets in Miden can be burned through various methods, such as rendering them unspendable by storing them in an unconsumable note, or sending them back to their original faucet for burning using it's dedicated function.

## Non-native asset

> All data structures not following the Miden asset model that can be exchanged.

Miden is flexible enough to support other asset models. For example, developers can replicate Ethereum’s ERC20 pattern, where fungible asset ownership is recorded in a single account. To transact, users send a note to that account, triggering updates in the global hashmap state.

## Conclusion

Miden’s asset model provides a secure, flexible, scalable, and privacy-preserving framework for representing and transferring value. By embedding asset information directly into accounts and supporting multiple asset types, Miden fosters a decentralized ecosystem where users maintain their privacy, control, transactions can scale efficiently, and censorship is minimized.
