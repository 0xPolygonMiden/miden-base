In this section, we show you how to execute transactions and send funds to another account using the Miden client through public notes. 

!!! important "Prerequisite steps"
    - You should have already followed all previous sections.
    - You should *not* have reset the state of your local client. 

## Create a second client

!!! tip
      - Remember to use the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/cli-reference/) for clarifications.
      - This is an alternative to the private off-chain P2P transactions article. 

We are going to use two clients to simulate two different remote users who do not share local state. We use two terminals with their own state (i.e. using their own `miden-client.toml`) to do this. 

One terminal runs the client from the [off-chain steps](p2p-private-offchain-txs.md) in the previous document while another terminal runs a new client.

1. Make sure your terminal from the previous section is open.

2. Open a new terminal and create a basic account:

      ```shell
      miden-client account new basic-immutable
      ```

3. List and view the accounts with the following command:

      ```shell
      miden-client account -l
      ```

## Transfer assets between accounts

1. Transfer some of the tokens we received from the faucet to our new account B. On the original client run:

    ```shell
    miden-client tx new p2id <regular-account-id-A> <regular-account-id-B> <faucet-account-id> 50 --note-type public
    ```

    !!! note
        The faucet account id can be found on the [Miden faucet website](https://ethdenver.polygonmiden.io/) under the title **Miden faucet**.

    This generates a Pay-to-ID (`P2ID`) note, containing `<amount>` assets, transferred from one account to the other. As the note is public, the second account can receive the necessary details by syncing with the node.

2. On the new client, sync the account: 

    ```shell
    miden-client sync # Make sure we have an updated view of the state
    ```

3. Check we received the public note details.

    ```sh
    miden-client input-notes list 
    ```

4. Have the second account consume the note.

    ```sh
    miden-client tx new consume-notes <regular-account-ID-B> <input-note-id> 
    ```

    !!! tip
        It's possible to use a short version of the note id: 7 characters after the `0x` is sufficient.

The second account has consumed the note and there should ne new assets in the account. Check by running the following command:

```sh
miden-client account show <account-ID> -v
```


