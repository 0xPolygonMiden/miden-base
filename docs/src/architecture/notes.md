# Notes
A note is a way of transferring assets between accounts. A note consists of a vault and a script as shown in the diagram below.

<p align="center">
    <img src="../diagrams/architecture/note/Note.png">
</p>

A note consists of:
* a set of assets stored in a vault.
* a script which must be executed in a context of some account to claim the assets.
* a set of inputs which are placed onto the stack before a note's script is executed.
* a serial number which can be used to break linkability between note and it's nullifier (~ exists when note was already consumed).

A note's vault is basically the same as an account's vault. However, unlike an account, a note has a single executable script. This script is also a root of a [Miden program MAST](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html). A script is always executed in the context of a single account, and thus, may invoke account's functions. A note script can take parameters (passed via the stack) as inputs. A note script does not have to call any of account's functions. More generally, a note's script can call zero or more of an account's function. A note's serial number identifies the note and this is needed to create the note's hash and nullifier. 
