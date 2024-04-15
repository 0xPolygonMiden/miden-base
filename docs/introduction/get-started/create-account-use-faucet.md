In this section, we show you how to create a new local Miden account and how to receive funds from the public Miden faucet website.

## Configure the Miden client

The Miden client facilitates interaction with the Miden rollup and provides a way to execute and prove transactions. 

!!! tip
      Check the [Miden client documentation](https://docs.polygon.technology/miden/miden-client/cli-reference/) for more information.

1. Clone the Miden client.

      ```shell
      git clone https://github.com/0xPolygonMiden/miden-client
      ```

2. Navigate to the client directory. 

      ```shell
      cd miden-client
      ```

3. Modify the configuration file at `./miden-client.toml` to point to the remote Miden node. 

      In the `[RPC]` section, replace the `endpoint = { host: }` field with the address provided by the Miden team.

      ```toml
      [rpc]
      endpoint = { protocol = "http", host = "<NODE_IP_ADDRESS>", port = 57291 }

      [store]
      database_filepath = "store.sqlite3"
      ```

4. Build and install the client using cargo:

      ```shell
      cargo install --features testing,concurrent --path .
      ```

5. Check you can use the help flag.

      ```shell
      miden-client --help
      ```

## Create a new Miden account

1. Create a new account called `basic-immutable` using the following command:

      ```shell
      miden-client account new basic-immutable
      ```

2. List all created accounts by running the following command:

      ```shell
      miden-client account -l
      ```

3. You should something like this:

      ![Result of listing miden accounts](../../img/get-started/miden-account-list.png)

Save the account ID for a future step.

## Request tokens from the public faucet

1. To request funds from the faucet navigate to the following website: [Miden faucet website](https://ethdenver.polygonmiden.io/).

2. Copy the **Account ID** printed by the `miden-client account -l` command in the previous step. 

3. Paste this id into the **Request test POL tokens** input field on the faucet website and click **Send me 100 tokens!**. 

4. After a few seconds your browser should download - or prompt you to download - a file called `note.mno` (mno = Miden note). It contains the funds the faucet sent to your address.

5. Save this file on your computer, you will need it for the next step. 

## Import the note into the Miden client

1. Import the note that you have received using the following commands: 

      ```shell
      miden-client input-notes -i <path-to-note>
      ```

2. You should see something like this:

      ```sh
      Succesfully imported note 0x0ff340133840d35e95e0dc2e62c88ed75ab2e383dc6673ce0341bd486fed8cb6
      ```

3. Now that the note has been successfully imported, you can view the note's information using the following command: 

      ```shell
      miden-client input-notes -l
      ```

4. You should see something like this:

      ![Result of viewing miden notes](../../img/get-started/note-view.png)

!!! tip "The importance of syncing"
      - As you can see, the listed note is lacking a `commit-height`. 
      - This is because you have received a note off-chain but have not yet synced your view of the rollup to check that the note is valid and exists at the rollup level. 
      - The nullified check at the operator level prevents double spends after a transaction takes place.
      - Hence, before consuming the note we will need to update our view of the rollup by syncing.

### Sync the client

Do this periodically to keep informed about any updates on the node by running the `sync` command:

```shell
miden-client sync
```

You will see something like this as output:

```sh
State synced to block 179672
```

## Consume the note & receive the funds

1. Now that we have synced the client, the input-note imported from the faucet should have a `commit-height` confirming it exists at the rollup level: 

      ```shell
      miden-client input-notes -l
      ```

2. You should see something like this:

      ![Viewing commit height info](../../img/get-started/commit-height.png)

3. Find your account and note id by listing both `accounts` and `input-notes`:

      ```shell
      miden-client account -l
      miden-client input-notes -l
      ```

4. Consume the note and add the funds from its vault to our account using the following command: 

      ```shell
      miden-client tx new consume-notes <Account-Id> <Note-Id>
      ```

!!! tip 
      - You only need to use the first 7 characters of the Note-Id.

5. View your updated account's vault containing the tokens sent by the faucet by running the following command: 

      ```shell
      miden-client account show <Account-Id> -v
      ```

6. You should now see your accounts vault containing the funds sent by the faucet. 

      ![Viewing account vault with funds](../../img/get-started/view-account-vault.png)
