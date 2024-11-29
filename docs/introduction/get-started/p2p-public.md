---
comments: true
---

In this section, we show you how to execute transactions and send funds to another account using the Miden client and [public notes](https://docs.polygon.technology/miden/miden-base/architecture/notes/#note-storage-mode). 

!!! important "Prerequisite steps"
    - You should have already followed the [prerequisite steps](prerequisites.md) and [get started](create-account-use-faucet.md) documents.
    - You should have *not* reset the state of your local client.

## Create a second client

!!! tip
      Remember to use the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/cli-reference/) for clarifications.

This is an alternative to the [private P2P transactions](p2p-private.md) process.

In this tutorial, we use two different clients to simulate two different remote users who don't share local state.

To do this, we use two terminals with their own state (using their own `miden-client.toml`).

1. Create a new directory to store the new client.

    ```sh
    mkdir miden-client-2
    cd miden-client-2
    ```

2. Initialize the client. This creates the `miden-client.toml` file line-by-line.

    ```sh
    miden init --rpc 18.203.155.106
    ```
    For the `--rpc` flag, enter the IP that the Miden team supplied.

3. On the new client, create a new [basic account](https://docs.polygon.technology/miden/miden-base/architecture/accounts/#account-types):

    ```shell
    miden new-wallet --mutable -s public
    ```

    We refer to this account as _Account C_. Note that we set the account's storage mode to `public`, which means that the account details are public and its latest state can be retrieved from the node.

4. List and view the account with the following command:

      ```shell
      miden account -l
      ```

## Transfer assets between accounts

1. Now we can transfer some of the tokens we received from the faucet to our new account C. Remember to switch back to `miden-client` directory, since you'll be making the txn from Account ID A.

    To do this, from the first client run:

    ```shell
    miden send --sender <basic-account-id-A> --target <basic-account-id-C> --asset 50::<faucet-account-id> --note-type public
    ```

    !!! note
        The faucet account id is `0xad904b3138d71d3e` and can also be found on the [Miden faucet website](https://testnet.miden.io/) under the title **Miden faucet**.

    This generates a Pay-to-ID (`P2ID`) note containing `50` tokens, transferred from one account to the other. As the note is public, the second account can receive the necessary details by syncing with the node.

2. First, sync the account on the new client.

    ```shell
    miden sync
    ```

3. At this point, we should have received the public note details. 

    ```sh
    miden notes --list
    ```

    Because the note was retrieved from the node, the commit height will be included and displayed.

4. Have account C consume the note.

    ```sh
    miden consume-notes --account <regular-account-ID-C> <input-note-id>
    ```

    !!! tip
        It's possible to use a short version of the note id: 7 characters after the `0x` is sufficient, e.g. `0x6ae613a`.

That's it!

Account C has now consumed the note and there should be new assets in the account:

```sh
miden account --show <account-ID> 
```

## Clear state

All state is maintained in `store.sqlite3`, located in the directory defined in the `miden-client.toml` file. 

To clear all state, delete this file. It recreates on any command execution.
