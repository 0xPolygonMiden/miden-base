In this section, we show you how to make off-chain transactions and send funds to another account using the Miden client. 

!!! important "Prerequisite steps"
    - You should have already followed all previous sections.
    - You should have *not* reset the state of your local client. 

## Create a second account

!!! tip
      Remember to use the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/cli-reference/) for clarifications.

1. Create a second account to send funds with. Previously, we created a `basic-immutable` (account A). Now, create `basic-immutable` (account B) using the following command:

      ```shell
      miden-client account new basic-immutable
      ```

2. List and view the newly created accounts with the following command:

      ```shell
      miden-client account -l
      ```

3. You should see two accounts:

      ![Result of listing miden accounts](../../img/get-started/two-accounts.png)

## Transfer assets between accounts

1. Now we can transfer some of the tokens we received from the faucet to our second account B. 

    To do this, run:

    ```shell
    miden-client tx new p2id <regular-account-id-A> <regular-account-id-B> <faucet-account-id> 50 --note-type private
    ```

    !!! note
        The faucet account id can be found on the [Miden faucet website](https://ethdenver.polygonmiden.io/) under the title **Miden faucet**.

    This generates a Pay-to-ID (`P2ID`) note containing `<amount>` assets, transferred from one account to the other. 

2. First, sync the accounts.

    ```shell
    miden-client sync # Make sure we have an updated view of the state
    ```

3. Get the second note id.

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

You should now see both accounts containing faucet assets with half the amount transferred from `Account A` to `Account B`.

!!! tip
    Remember. The original amount was 100 POL.

Check the second account:

```shell
miden-client account show <regular-account-ID-B> -v # Show account B's vault assets (50 fungible tokens)
```

![Result of listing miden accounts](../../img/get-started/account-b.png)

Check the original account:

```sh
miden-client account show <regular-account-ID-A> -v # Show account A's vault assets (950 fungible tokens)
```

![Result of listing miden accounts](../../img/get-started/account-a.png)

## Clear state

All state is maintained in `store.sqlite3`, located in the directory defined in the `miden-client.toml` file. 

To clear all state, delete this file. It recreates on any command execution.

## Congratulations! 

You have successfully configured and used the Miden client to interact with a Miden rollup and faucet. 

You have performed basic Miden rollup operations like sending transactions, generating and consuming notes.

For more information on the Miden client, refer to the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/).
