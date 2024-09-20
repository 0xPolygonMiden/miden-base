---
comments: true
---

## Context overview

Miden assembly program execution, the code the transaction kernel runs, spans multiple isolated contexts. An execution context defines its own memory space which is inaccessible from other execution contexts. Note scripts cannot directly write to account data which should only be possible if the account exposes relevant functions.

## Specific contexts

The kernel program always starts executing from a root context. Thus, the prologue sets the memory for the root context. To move execution into a different context, the kernel invokes a procedure using the `call` or `dyncall` instruction. In fact, any time the kernel invokes a procedure using the `call` instruction, it executes in a new context.

While executing in a note, account, or tx script context, the kernel executes some procedures in the kernel context, which is where all necessary information was stored during the prologue. The kernel switches context via the `syscall` instruction. The set of procedures invoked via the `syscall` instruction is limited by the [transaction kernel API](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/kernels/transaction/api.masm). When the procedure called via `syscall` returns, execution moves back to the note, account, or tx script where it was invoked.

## Context switches

<center>
![Transaction contexts](../../img/architecture/transaction/transaction-contexts.png)
</center>

The above diagram shows different context switches in a simple transaction. In this example, an account consumes a [P2ID](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/P2ID.masm) note and receives the asset into its vault. As with any MASM program, the transaction kernel program starts in the root context. It executes the prologue and stores all necessary information into the root memory.

The next step, note processing, starts with a `dyncall` which invokes the note script. This command moves execution into a different context **(1)**. In this new context, the note has no access to the kernel memory. After a successful ID check, which changes back to the kernel context twice to get the note inputs and the account id, the script executes the `add_note_assets_to_account` procedure.

```arduino
# Pay-to-ID script: adds all assets from the note to the account, assuming ID of the account
# matches target account ID specified by the note inputs.
# ...
begin

    ... <check correct ID>

    exec.add_note_assets_to_account
    # => [...]
end
```

The procedure cannot simply add assets to the account, because it is executed in a note context. Therefore, it needs to `call` the account interface. This moves execution into a second context - account context - isolated from the note context **(2)**.

```arduino
#! Helper procedure to add all assets of a note to an account.
#! ...
proc.add_note_assets_to_account
    ...

    while.true
        ...

        # load the asset and add it to the account
        mem_loadw call.wallet::receive_asset
        # => [ASSET, ptr, end_ptr, ...]
        ...
    end
    ...
end
```

The [wallet](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/miden/contracts/wallets/basic.masm) smart contract provides an interface that accounts use to receive and send assets. In this new context, the wallet calls the `add_asset` procedure of the account API.

```arduino
export.receive_asset
    exec.account::add_asset
    ...
end
```

The [account API](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/miden/account.masm#L162) exposes procedures to manage accounts. This particular procedure that was called by the wallet invokes a `syscall` to return back to the root context **(3)**, where the account vault is stored in memory (see prologue). Procedures defined in the [Kernel API](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/kernels/transaction/api.masm) should be invoked with `syscall` using the corresponding [procedure offset](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/miden/kernel_proc_offsets.masm) and the `exec_kernel_proc` kernel procedure.

```arduino
#! Add the specified asset to the vault.
#! ...
export.add_asset
    exec.kernel_proc_offsets::account_vault_add_asset_offset
    syscall.exec_kernel_proc
end
```

Now, the asset can be safely added to the vault within the kernel context, and the note can be successfully processed.

<br/>
