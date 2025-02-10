# Transaction

A `Transaction` in Miden is the state transition of a single account. A `Transaction` takes as input a single [account](accounts.md) and zero or more [notes](notes.md), and outputs the same account with an updated state, together with zero or more notes. `Transaction`s in Miden are Miden VM programs, their execution resulting in the generation of a zero-knowledge proof.

Miden's `Transaction` model aims for the following:

- **Parallel transaction execution**: Accounts can update their state independently from each other and in parallel.
- **Private transaction execution**: Client-side `Transaction` proving allows the network to verify `Transaction`s validity with zero knowledge.

![Transaction diagram](../img/architecture/transaction/transaction-diagram.png)

Compared to most blockchains, where a `Transaction` typically involves more than one account (e.g., sender and receiver), a `Transaction` in Miden involves a single account. To illustrate, Alice sends 5 ETH to Bob. In Miden, sending 5 ETH from Alice to Bob takes two `Transaction`s, one in which Alice creates a note containing 5 ETH and one in which Bob consumes that note and receives the 5 ETH. This model removes the need for a global lock on the blockchain's state, enabling Miden to process `Transaction`s in parallel.

## Transaction lifecycle

Every `Transaction` describes the process of an account changing its state. This process is described as a Miden VM program, resulting in the generation of a zero-knowledge proof.

![Transaction execution process](../img/architecture/transaction/transaction-execution-process.png)

### Transaction execution flow

`Transaction`s are being executed in a well-defined sequence, in which several note and transaction scripts can call the account interface to interact with it.

![Transaction execution flow](../img/architecture/transaction/transaction-program.png)

1. **Prologue**: On-chain commitments are validated against provided data.
2. **Note processing**: Input notes are executed sequentially against the account, following a  sequence defined by the executor.
    - Notes must be consumed fully (all assets must be transferred to the account or to another note).
    - The note script must be executed in full with the provided note inputs and transaction arguments. Transaction arguments can be injected by the executor for each note at runtime.
3. **Transaction script execution (optional)**: A `Transaction` script, if present, is executed.
    - This script can sign the `Transaction` or directly interact with the account without using notes.
4. **Epilogue**: Execution completes, resulting in an updated account state and a generated zero-knowledge proof. The validity of the state change is being checked
    - The account state is updated.
    - New notes are created (optional), transferring assets from the account to the newly created notes.
    - The `Nonce` must be incremented, to allow transaction authentication. Also the net sum of all involved assets mus be 0 - if the account is no faucet.

The proof together with the corresponding data needed for verification and updates on the global state can then be submitted and processed by the network.

### Transaction inputs

A `Transaction` is restricted to only one native account. The state of the native account can be updated. However, also account states of other, foreign accounts can be read during the transaction and the information can be processed. For example, a note script can read the state of an oracle account, e.g., the price of an asset, to dynamically adjust the note's spent conditions.

To execute a `Transaction`, the executor must have complete knowledge of the native account state, the authenticated data of the foreign accounts to be read, and the notes used as `Transaction` inputs.

- **Notes**: A `Transaction` can only consume notes if the full note data is known. For private notes, the data cannot be fetched from the blockchain and must be received otherwise.
- **Transaction arguments**: For every note, the executor can inject transaction arguments that are present at runtime. If the note script - and therefore the note creator - allows, the note script can read those arguments.
- **Foreign account data**: Any foreign account data accessed during a `Transaction`, whether private or public, must be available beforehand. There is no need to know the full account storage, but the data necessary for the `Transaction`, e.g., the key/value pair that is read and the corresponding storage root.
- **Blockchain state**: The current reference block and information about the notes database used to authenticate input notes must be retrieved from the Miden operator before execution. Usually, notes to be consumed in a `Transaction` must have been created before the reference block.


> **Info**
> - Usually, notes that are consumed in a `Transaction` must be recorded on-chain in order for the `Transaction` to succeed. However, in Miden there is the concept of ephemeral notes that can be consumed in a `Transaction` before being registered on-chain. This allows the executor to consume notes before they reach the blockchain which is useful for example to achieve sub-second orders in DEXs.
> - There is no nullifier check during a `Transaction`. Nullifiers are checked by the Miden operator during `Transaction` verification. So at the `Transaction` level, there is "double spending." If a note was already spent, i.e. there exists a nullifier for that note, the whole `Transaction` will fail when submitted to the network.

### Notes and transaction scripts

[Note scripts](note.md/script) can invoke the account interface during execution. They can push assets into the account's vault, create new notes, set a transaction expiration, and read from or write to the account’s storage. Any method they call must be explicitly exposed by the account interface. Note scripts can also invoke methods of foreign accounts to read their state.

`Transaction` scripts execute after all note scripts and are optional. Unlike note scripts, which are defined by the note creator, `Transaction` scripts are defined by the executor. For example, they allow the executor to authenticate the transaction via the account’s authentication component. However, the executor can invoke any exposed method. This enables actions such as creating notes, modifying account storage, or, if the account is a faucet, minting new tokens. `Transaction` scripts can also invoke methods of foreign accounts to read their state.

## Basic examples - consuming and creating a P2ID note

To illustrate the `Transaction` protocol, we provide two examples for a basic `Transaction`. We will reference to the existing Miden `Transaction` kernel - the reference implementation of the protocol - and to the methods in Miden Assembly.

### Creating a P2ID note

Let's assume an account A that wants to consume a P2ID note to receive the assets contained in that note.

Account A has two methods explicitly exposed - `receive_asset` and the authentication method `auth_tx_rpo_falcon512`. For illustrative purposes, we use the standard P2ID note as defined in the `miden-lib`. Some account methods like `account::get_id` are always exposed.

The executor starts with fetching and preparing all the input data to the `Transaction`. First, it retrieves the global inputs and block data of the last recent block. Global inputs include the block hash, account ID, initial account hash, and nullifier commitment. The block data includes the chain root and data to verify the block hash. This information is needed to authenticate the native account's state and that the P2ID exists on-chain.

Together with the full account and note data, the executor prepares a data store to start the `Transaction` execution.

In the prologue of the transaction the data is being authenticated and the account and notes are being loaded into the kernels memory.

After the prologue the note script of the P2ID note is being loaded and executed. The script starts by reading the note inputs `note::get_inputs` - in our case the account Id of the intended target account. Then it checks if the account Id provided by the note inputs equal the account Id of the executing account. This is the first time, the note invokes a method exposed by the account `account::get_id`.

If the check passes, the note script pushes the assets it holds into the account's vault. For every asset the note contains, the script calls the `receive_asset` method exposed but the account's wallet component. For security reasons, `receive_asset` calls `account::add_asset`, which can not be called within the note context itself.

After the assets are stored in the account's vault, the transaction script is being executed. The script calls `auth_tx_rpo_falcon512` which is explicitly exposed in the account interface. The method is used to verify a provided signature against a public key stored in the account's storage and a commitment to this specific transaction. If the signature can be verified, the method increments the nonce. 

The epilogue finalized the transaction by computing the final account hash, asserts the nonce increment and checks that no assets were created or destroyed in the transaction - that means the net sum of all assets must stay the same.


### Consuming a P2ID note

<Paul Henry, can you try to give an example of how to create a P2ID note?>

## Transaction types

There are two types of `Transaction`s in Miden: **local transactions** and **network transactions** [not yet implemented].

### Local transaction

Users transition their account's state locally using the Miden VM and generate a `Transaction` proof that can be verified by the network, which we call **client-side proving**. The network then only has to verify the proof and to change the global parts of the state to apply the state transition.

They are useful, because:

1. They enable privacy as neither the account state nor account code are needed to verify the zero-knowledge proof. Public inputs are only commitments and block information that are stored on-chain.
2. They are cheaper (i.e., lower in fees) as the execution of the state transition and the generation of the zero-knowledge proof are already made by the users. Hence **privacy is the cheaper option on Miden**.
3. They allow arbitrarily complex computation to be done. The proof size doesn't grow linearly with the complexity of the computation. Hence there is no gas limit for client-side proving.

Client-side proving or local transactions on low-power devices can be slow, but Miden offers a pragmatic alternative: **delegated proving**. Instead of waiting for complex computations to finish on your device, you can hand off proof generation to a service, ensuring a consistent 1-2 second proving time, even on mobile.

### Network transaction

The Miden operator executes the `Transaction` and generates the proof. Miden uses network `Transaction`s for smart contracts with public shared state. This type of `Transaction` is quite similar to the ones in traditional blockchains (e.g., Ethereum).

They are useful, because:

1. For public shared state of smart contracts. Network `Transaction`s allow orchestrated state changes of public smart contracts without race conditions.
2. Smart contracts should be able to be executed autonomously, ensuring liveness. Local `Transaction`s require a user to execute and prove, but in some cases a smart contract should be able to execute when certain conditions are met.
3. Clients may not have sufficient resources to generate zero-knowledge proofs.

![Local vs network transactions](../img/architecture/transaction/local-vs-network-transaction.png)

The ability to facilitate both, local and network `Transaction`s, **is one of the differentiating factors of Miden** compared to other blockchains. Local `Transaction` execution and proving can happen in parallel as for most `Transaction`s there is no need for public state changes. This increases the network's throughput tremendously and provides privacy. Network `Transaction`s on the other hand enable autonomous smart contracts and public shared state.

### Transaction limits

Currently the protocol limits the number of notes that can be consumed or produced in a transaction to `1K` each. That means on the other hand, in one transaction, an application can serve up to 2000 different user requests like deposits or withdrawals into a pool. 

A simple transaction currently takes about 1-2 seconds on a MacBook Pro. It takes around `90K` cycles to create the proof, whereas the signature verification step is the dominant cost. 


---

> **More info**
> - One of the main reasons for separating execution and proving steps is to allow _stateless provers_; i.e., the executed `Transaction` has all the data it needs to re-execute and prove a `Transaction` without database access. This supports easier proof-generation distribution.
>
> - Not all `Transaction`s require notes. For example, the owner of a faucet can mint new tokens using only a `Transaction` script, without interacting with external notes.
>
> - It is possible to set `Transaction` expiration heights and in doing so, to define a block height until a `Transaction` should be included into a block. If the `Transaction` is expired, the resulting account state change is not valid and the `Transaction` cannot be verified anymore.
>
> - Note and `Transaction` scripts can read the state of foreign accounts during execution. This is called foreign procedure invocation. For example, the price of an asset for the **Swap** script might depend on a certain value stored in the oracle account.
>
> - An example of the right usage of `Transaction` arguments is the consumption of a **Swap** note. Those notes allow asset exchange based on predefined conditions. Example:
    - The note's consumption condition is defined as "anyone can consume this note to take X units of asset A if they simultaneously create a note sending Y units of asset B back to the creator."
    - If an executor wants to buy only a fraction `(X-m)` of asset A, they provide this amount via transaction arguments. The executor would provide the value `m`.
    - The note script then enforces the correct transfer:
        - A new note is created returning `Y-((m*Y)/X)` of asset B to the sender.
        - A second note is created, holding the remaining `(X-m)` of asset A for future consumption.
