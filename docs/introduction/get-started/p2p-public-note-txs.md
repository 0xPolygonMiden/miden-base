In this section, we show you how to execute transactions and send funds to another account using the Miden client through public notes. 

!!! important "Prerequisite steps"
    - You should have already followed all previous sections.
    - You should have *not* reset the state of your local client. 

## Create a second client

!!! tip
      Remember to use the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/cli-reference/) for clarifications.

This is an alternative to the private offchain P2P transactions article. For this tutorial, we will utilize two different clients to simulate two different remote users who don't share local state. To do this, we can have two terminals with their own state (using their own `miden-client.toml`). One terminal can run the client from the previous article, and a new terminal can run a new client.

1. On the new client, let's create a new basic account:

      ```shell
      miden-client account new basic-immutable
      ```

2. List and view the accounts with the following command:

      ```shell
      miden-client account -l
      ```

## Transfer assets between accounts

1. Now we can transfer some of the tokens we received from the faucet to our new account B. 

    To do this, from the first client run:

    ```shell
    miden-client tx new p2id <regular-account-id-A> <regular-account-id-B> <faucet-account-id> 50 --note-type public
    ```

    !!! note
        The faucet account id can be found on the [Miden faucet website](https://ethdenver.polygonmiden.io/) under the title **Miden faucet**.

    This generates a Pay-to-ID (`P2ID`) note containing `<amount>` assets, transferred from one account to the other. As the note is public, the second account can receive the necessary details by syncing with the node.

2. First, sync the account on the new client.

    ```shell
    miden-client sync # Make sure we have an updated view of the state
    ```

3. At this point, we should have received the public note details.

    ```sh
    miden-client input-notes list 
    ```

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
