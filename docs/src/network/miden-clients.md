# Miden Clients
Users use Miden Clients to interact in the network. The backend of any wallet that is used in Miden will be a Miden Client. Miden Clients consist of several components.

* Transaction Prover
* Signature module
* Wallet interface
* Wallet database

[We need a diagram to show the Miden Client]

## Transaction Prover
The Transaction Prover is able to execute transactions and create transaction execution proofs. It runs a Transaction Kernel at its heart.

## Signature module
[Unclear if this is a separate module]

## Wallet interface
At the beginning we only have a basic wallet interface which we implement for the testnet. It is rather simplistic.

The interface defines three methods:

```
receive_asset
send_asset
auth_tx
```

The first two of the above methods should probably be an interface on their own, and we should recommend that most accounts implement these methods.

The goal is to provide a wallet with the following capabilities:

The wallet is controlled by a single key. The signature scheme is assumed to be Falcon. However, sending assets to the wallet does not require knowing which signature scheme is used by the recipient. The user can send, receive, and exchange assets stored in the wallet with other users. All operations (including receiving assets) must be authenticated by the account owner.

Interface method description
Below, we provide high-level details about each of the interface methods.

### `receive_asset method`
The purpose of this method is to add a single asset to an account's vault. Pseudo-code for this method could look like so:

```
receive_asset(asset)
    self.add_asset(asset)
end
```

In the above, `add_asset` is a kernel procedure `miden::single_account::account::add_asset` of the Tx Kernel.

Note: this method does not increment account nonce. The nonce will be incremented in auth_tx method described below. Thus, receiving assets requires authentication.

### `send_asset method`
The purpose of this method is to create a note which sends a single asset to the specified recipient. Pseudo-code for this method could look like so:

```
send_asset(asset, recipient)
    self.remove_asset(asset)
    tx.create_note(recipient, asset)
end
```

In the above, `remove_asset` is a kernel procedure `miden::single_account::account::remove_asset` and `create_note` is a kernel procedure `miden::single_account::tx::create_note`, both in the Tx Kernel.

`recipient` is a partial hash of the created note computed outside the VM as `hash(hash(hash(serial_num), script_hash), input_hash)`. This allows computing note hash as `hash(recipient, vault_hash)` where the `vault_hash` can be computed inside the VM based on the specified asset.

Note: this method also does not increment account nonce. The nonce will be incremented in auth_tx method described below. Thus, sending assets requires authentication.

### `auth_tx method`
The purpose of this method is to authenticate a transaction. For the purposes of this method we make the following assumptions:

Public key of the account is stored in account storage at index 0.
To authenticate a transaction we sign `hash(account_id || account_nonce || input_note_hash || output_note_hash)` using Falcon signature scheme. Pseudo-code for this method could look like so:

```
auth_tx()
    # compute the message to sign
    let account_id = self.get_id()
    let account_nonce = self.get_nonce()
    let input_notes_hash = tx.get_input_notes_hash()
    let output_notes_hash = tx.get_output_notes_hash()
    let m = hash(account_id, account_nonce, input_notes_hash, output_notes_hash)

    # get public key from account storage and verify signature
    let pub_key = self.get_item(0)
    falcon::verify_sig(pub_key, m)

    # increment account nonce
    self.increment_nonce()
end
```

It is assumed that the signature for `falcon::verify_sig procedure` will be provided non-deterministically via the advice provider. Thus, the above procedure can succeed only if the prover has a valid Falcon signature over `hash(account_id || account_nonce || input_note_hash || output_note_hash)` for the public key stored in the account.

All procedures invoked as a part of this method, except for `falcon::verify_sig` have equivalent kernel procedures defined in the Tx Kernel. We assume that `falcon::verify_sig` is a part of Miden standard library.

## Wallet database
[Unclear yet how this database looks like. It should at least have `assets/vault`, `code`, `nonce`, `storage`]
