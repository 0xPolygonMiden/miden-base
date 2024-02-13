# Getting started

This tutorial guides you through the process of connecting to a remote Miden node using the Miden client. The Miden node processes transactions and creates blocks for the Miden rollup. The Miden client provides a way to execute and prove transactions, facilitating the interaction with the Miden rollup. By the end of this tutorial, you will be able to configure the Miden client, connect to a Miden node, and perform basic operations like sending transactions, generating and consuming notes.

## Prerequisites

Before starting, ensure you have the following:

- **Rust Installed:** You must have the Rust programming language installed on your machine. If you haven't installed Rust, you can download it from [the Rust website](https://www.rust-lang.org/learn/get-started).
- **Node IP Address:** Obtain the IP address of the running Miden node. This information can be acquired by contacting one of the Miden engineers.
- **Miden client Installation:** You need to install the [Miden client](https://github.com/0xPolygonMiden/miden-client) and configure it to point to the remote node.

## Step 1: Configuring the Miden client

1. **Download the Miden client:** First, download the Miden client from its repository. Use the following command:

   ```shell
   git clone https://github.com/0xPolygonMiden/miden-client
   ```

2. **Navigate & Configure the client:** Navigate to the client directory and modify the configuration file to point to the remote Miden node. You can find the configuration file at `./miden-client.toml`. In the `[RPC]` section replace the `endpoint = { host: }` field with the address provided by the Miden engineer.

   ```shell
   cd miden-client
   ```

   Configuration file example:

   ```toml
    [rpc]
    endpoint = { protocol = "http", host = "<NODE_IP_ADDRESS>", port = 57291 }

    [store]
    database_filepath = "store.sqlite3"
   ```

3. **Build & install the Client:** install the client using cargo:

   ```shell
   cargo install --features testing,concurrent --path .
   ```

   you should now be able to use the following command:

   ```shell
   miden-client --help
   ```

## Step 2: Setting-up the Miden client

1. **Creating new accounts:** To be able to interact with the Miden node we will need to generate accounts. For this example we will be generating 3 accounts: `basic-immutable` account A, `basic-immutable` account B and a `fungible-faucet`. You can generate new accounts using the following commands:

   ```shell
   miden-client account new basic-immutable
   miden-client account new basic-immutable
   miden-client account new fungible-faucet [...]
   ```

   Please refer to the documentation of the CLI

2. **Listing accounts:** To view the newly created accounts we can run the following command:

   ```shell
   miden-client account -l
   ```

   We should now see 3 available accounts listed:
   - `basic-immutable` account A
   - `basic-immutable` account B
   - `fungible-faucet` account

3. **Syncing node state:** The client needs to periodically query the node to receive updates about entities that might be important in order to run transactions. The way to do so is by running the `sync` command:

   ```shell
   miden-client sync
   ```

## Step 3: Minting an asset

Since we have now synced our local view of the blockchain and have account information, we are ready to execute and submit tranasctions. For a first test, we are going to mint a fungible asset for a regular account.

```shell
miden-client tx new mint <regular-account-id-A> <faucet-account-id> 1000
```

This will execute, prove and submit a transaction that mints assets to the node. The account that executes this transaction will be the faucet as was defined in the node's configuration file. In this case, it is minting `1000` fungible tokens to `<regular-account-id-A>`.

This will add a transaction and an output note (containing the minted asset) to the local store in order to track their lifecycles. You can display them by running `miden-client tx list` and `miden-client input-notes list` respectively. If you do so, you will notice that they do not show a `commit height` even though they were submitted to the operator. This is because our local view of the network has not yet been updated. After updating it with a `sync`, you should see the height at which the transaction and the note containing the asset were committed. This will allow us to prove transactions that make use of this note, as we can compute valid proofs that state that the note exists in the blockchain.

## Step 4: Consuming the note

After creating the note with the minted asset, the regular account can now consume it and add the tokens to its vault. You can do this the following way:

```bash
miden-client tx new consume-note <regular-account-id-A> <input-note-id>
```

This will consume the input note, which you can get by listing them as explained in the previous step. You will now be able to see the asset in the account's vault by running:

```bash
miden-client account show <regular-account-id-A> -v
```

## Step 5: Transferring assets between accounts

Some of the tokens we minted can now be transferred to our second regular account. To do so, you can run:

```shell
miden-client sync # Make sure we have an updated view of the state
miden-client tx new p2id <regular-account-id-A> <regular-account-id-B> <faucet-account-id> 50 # Transfers 50 tokens to account ID B
```

This will generate a Pay-to-ID (`P2ID`) note containing 50 assets, transferred from one regular account to the other. If we sync, we can now make use of the note and consume it for the receiving account:

```shell
miden-client sync # Make sure we have an updated view of the state
miden-client tx new consume-note <regular-account-ID-B> # Consume the note
```

That's it! You will now be able to see `950` fungible tokens in the first regular account, and `50` tokens in the remaining regular account:

```shell
miden-client account show <regular-account-ID-B> -v # Show account B's vault assets (50 fungible tokens)
miden-client account show <regular-account-ID-A> -v # Show account A's vault assets (950 fungible tokens)
```

### Clearing the state

All state is maintained in `store.sqlite3`, located in the directory defined in the `miden-client.toml` file. In case it needs to be cleared, the file can be deleted; it will later be created again when any command is executed.

## Conclusion

Congratulations! You have successfully configured and used the Miden client to interact with a Miden node. With these steps, you can perform basic Miden rollup operations like sending transactions, generating and consuming notes.

For more informations on the Miden client, refer to the [Readme of the Miden Client](https://github.com/0xPolygonMiden/miden-client)

For more informations on the Miden rollup, refer to the [Miden documentation](https://0xpolygonmiden.github.io/miden-base/introduction.html).
