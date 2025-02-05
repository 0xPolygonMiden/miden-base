# Transaction

`Transaction`s are a mechanism by which the state of the chain is updated. That is, all changes in account and note states result from executing `Transaction`s. In Miden, `Transaction`s can originate either from the users or from the network itself.

Miden's `Transaction` model aims for the following:

- **Parallel transaction execution**: Accounts can change their state independently from each other and in parallel.
- **Private transaction execution**: Client-side transaction execution and prooving allows the network to verify transactions with zero knowledge.

## What is the purpose of a transaction?

In Miden, a `Transaction` represents the state transition of a single account. A `Transaction` takes a single [account](accounts.md) and zero or more [notes](notes.md), and outputs the same account in a new state, together with zero or more [notes]. `Transaction`s in Miden are executed as Miden VM programs, which means that a zero-knowledge proof is generated as a result of each `Transaction`.

![Transaction diagram](../img/architecture/transaction/transaction-diagram.png)

## Transaction types

There are two types of transactions in Miden: **local transactions** and **network transactions**.

In **local transactions**, clients apply the account's state transition locally and send a transaction proof to the network. The network only verifies the proof and changes the global parts of the state.

They are useful, because:

1. They are cheaper (i.e., lower in fees) as the execution of the state transition and the generation of the zk-proof are already made by the users. Hence **privacy is the cheaper option on Miden**.
2. They allow arbitrarly complex computation to be done. The proof size doesn't grow linearly with the complexity of the computation. Hence there is no gas limit for client-side execution and proving.
3. They enable privacy as neither the account state nor account code are needed to verify the zk-proof. Public inputs are only commitments and block information that are stored on-chain.

In **network transactions**, the Miden operator executes the transaction and generates the proof. Miden uses network transactions for smart contracts with public shared state. This type of `Transaction` is quite similar to the ones in traditional blockchains (e.g., Ethereum).

They are useful, because:

1. Smart contracts should be able to be executed autonomously. Local transactions require a user to execute and prove. In some cases a smart contract should be able to execute when certain conditions are met or simply when it is called. Network transactions ensure liveness of smart contracts.
2. For public shared state of smart contracts. Network transactions allow orchestrated state changes of public smart contracts without race conditions. See <here> for an in-depth explanation of public share state on Miden.
3. Clients may not have sufficient resources to generate zk-proofs.

![Local vs network transactions](../img/architecture/transaction/local-vs-network-transaction.png)

The ability to facilitate both, local and network transactions, is what differentiates Miden from other blockchains. Local transaction execution and proving can happen in parallel as for most transactions there is no need for public state changes. This increases the network's throughput tremendously and provides privacy. Network transactions on the other hand enable autonomous smart contracts and public shared state.

## Transaction lifecycle

Every `Transaction` describes the process of an account changing its state. This process described as a Miden-VM program resulting in a zero-knowledge proof of correct execution.

![Transaction execution process](../img/architecture/transaction/transaction-execution-process.png)

In order to execute a transaction locally, the user needs to know the complete state of the account and of notes that serve as inputs to the transaction. That means, one can only consume private notes in a transaction, if the full private note data is known. The same goes for foreign account data, whether it is private or public. If data from a foreign account is read during a transaction, the data must be present before the local transaction execution. Together with the account and note data, information about the current state of the blockchain needs to be fetched from the Miden operator. This entails the current `BlockHeader` and the `ChainMMR` which authenticates inputed notes during transaction execution.

> **Info**
> - `InputNotes` must not be recorded on-chain in order for the transaction to succeed. In Miden there is the concept of ephemeral notes that can be consumed in a transaction before registered on-chain. <link to notes chapter ephemeral notes>
> - There is no nullifier-check during a transaction. Nullifiers are checked by the Miden operator during transaction verification. So at the transaction level, there is "double spending." If a note was already spent, i.e. there exists a nullifier for that note, the whole transaction will fail when submitted to the network.

Transaction execution starts with a prologue where on-chain commitments are checked against the provided data. Then, all notes are being executed sequentially against the account. The executor can define the sequence of the `InputNotes` and notes must be consumed fully. That means all assets must be consumed into the account and the note script must be executed fully given the provided [note inputs](notes.md/inputs) and transaction arguments. Let's unfold what that means:

When a note is being consumed in a transaction, the note script is being executed against the executing account. That way, the note script can call exposed functions of the account interface. For example, a basic wallet account exposes `receive_asset`. When a note calls this function in its script, the assets that are contained in the note are being transferred to the account. In most cases, the note creator doesn't want a note and its assets to be consumable by all the accounts. That is why, the note creator can impose additional checks. Since the executing account needs to execute the full note script in order to consume the note, it can not circumvent the checks set by the note creator. For example, the P2ID note script checks that the ID of the executing account matches the AccountID provided by the [note inputs](notes.md/inputs). But also the executor can inject arguments to the note script execution called `TransactionArguments`. For example, a basic SWAP note is defined as "Anyone can consume this note and take the X assets A on it if during execution another note is created to send Y of asset B back to the sender". Now, the executor might not want to "buy" X of asset A, but only X-m - a smaller portion. If the note creator is fine with also selling less of A for the same price, this can be encoded in the note script. The executor will then provide the amount willing to buy via the `TransactionArguments` and the note script forces the executor to create two new notes - one back to the sender containing Y-((m*Y)/X) of asset B (assuming that X/Y = m/n) and another one, a copy of the initial node containing X-m of asset A for other accounts to consume. Note creators can define any arbitrary logic into their note scripts which is executed during transaction execution. However, note scripts can only call functions of the account that are exposed via the account code. Most internal account functions, manipulating the vault or storage, are shielded from note scripts.

After all notes are being consumed, the transaction script is next in line. The transaction script is an optional piece of code defined by the executor. Usually, it is used to sign the transaction, it can also be used to interact with an account without using notes. For example, the owner of a faucet can `mint` new tokens using only a transaction script.

After all note scripts and the optional transaction script have been processed, the account gets being updated in the transaction's epilogue. If the transaction also creates new notes, they are created and assets are moved from the account to the corresponding `OutputNotes`. Now, the transaction execution is complete resulting in an account with a new state and the corresponding `OutputNotes`.

![Transaction execution process](../img/architecture/transaction/transaction-program.png)

If the transaction succeeds, a proof is generated of the correct transaction execution. This proof together with the corresponding data needed for verification and updates on the global state can then be submitted and processed by the network.

> **Info**
> - One of the main reasons for separating out the execution and proving steps is to allow _stateless provers_; i.e., the executed transaction has all the data it needs to re-execute and prove a transaction without database access. This supports easier proof-generation distribution.

More notable transaction features:

- It is possible to set transaction expiration heights and in doing so, to define a block height until a transaction should be included into a block. If the transaction is expired, the resulting account state change is not valid and the transaction can not be verified anymore.
- Note and transaction scripts can read the state of foreign accounts during execution. This is called foreign procedure invocation. For example the price of an asset for the SWAP script might depend on a certain value stored in the oracle account.
