In this section, we show you how to execute transactions and send funds to another account using the Miden client through [public notes](https://docs.polygon.technology/miden/miden-base/architecture/notes/#note-storage-mode). 

!!! important "Prerequisite steps"
    - You should have already followed all previous sections.
    - You should have *not* reset the state of your local client. 

## Create a second client

!!! tip
      Remember to use the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/cli-reference/) for clarifications.

This is an alternative to the private off-chain P2P transactions article. For this tutorial, we will utilize two different clients to simulate two different remote users who don't share local state. To do this, we will have two terminals with their own state (using their own `miden-client.toml`).

1. First, let's create a new directory to store the new client

      ```shell
      mkdir miden-client-2
      cd miden-client-2

      miden-client init # Create the miden-client.toml
      ```

2. On the new client, let's create a new [basic account](https://docs.polygon.technology/miden/miden-base/architecture/accounts/#account-types):

      ```shell
      miden-client account new basic-mutable -s on-chain
      ```

We will refer to this account by _Account B_. Note that we set the account's storage mode to `on-chain`, which means that the account details will be public and its latest state can be retrieved from the node.

3. List and view the account with the following command:

      ```shell
      miden-client account -l
      ```

## Transfer assets between accounts

1. Now we can transfer some of the tokens we received from the faucet to our new account B. 

    To do this, from the first client run:

    ```shell
    miden-client tx new p2id <basic-account-id-A> <basic-account-id-B> <faucet-account-id> 50 --note-type public
    ```

    !!! note
        The faucet account id can be found on the [Miden faucet website](https://testnet.miden.io/) under the title **Miden faucet**.

    This generates a Pay-to-ID (`P2ID`) note containing `<amount>` assets, transferred from one account to the other. As the note is public, the second account can receive the necessary details by syncing with the node.

2. First, sync the account on the new client.

    ```shell
    miden-client sync # Make sure we have an updated view of the state
    ```

3. At this point, we should have received the public note details. 

    ```sh
    miden-client input-notes list 
    ```

    Because the note was retrieved from the node, the commit height will be included and displayed.

4. Have the second account consume the note.

    ```sh
    miden-client tx new consume-notes <regular-account-ID-B> <input-note-id> 
    ```

    !!! tip
        It's possible to use a short version of the note id: 7 characters after the `0x` is sufficient.

That's it! 

The second account will have now consumed the note and should have new assets in the account:

```sh
miden-client account show <account-ID> -v
```
