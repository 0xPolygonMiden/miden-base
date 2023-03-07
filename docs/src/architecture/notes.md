# Notes
A note is a way of transferring assets between accounts. A note consists of a vault and a script as shown in the diagram below.

<p align="center">
    <img src="../diagrams/protocol/note/Note.png">
</p>

A note's vault is basically the same as an account's vault. However, unlike an account, a note has a single executable script. This script is also a root of a Miden program MAST. A script is always executed in the context of a single account, and thus, may invoke account's functions. For example, a simple script could look like this:

```
begin
  push.123
  call.0x123456    # this may correspond to `function1`
end
```

The above will push value 123 onto the stack and then call a function with MAST root 0x123456 (assuming such function exists in the account). A few other things to note about note scripts:

* A note script can take parameters (passed via the stack). This could be one way to pass such environment variables as block hash, account ID etc. to a script.
* A note script does not have to call any of account's functions. More generally, a note's script can call zero or more of an account's function.
* A script is not explicitly tied to any specific account. For example, the above script can be executed against any account which exposes a function with root 0x123456. However, it is possible to tie scripts to accounts explicitly. For example, if we wanted to make sure a script can be executed against an account with ID 0x9876, we could do something like this:

```
use std.account;

begin
  exec.account::get_id
  push.0x9876
  eqw
  assert
  push.123
  call.0x123456
end
```

Executing this script will have give the same result as executing the earlier simpler script, but this script cannot be executed against any other account.
