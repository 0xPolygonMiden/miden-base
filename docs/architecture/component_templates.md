# Account component templates

An account component template provides a general description of an account component. It encapsulates all the information needed to initialize and manage the component.

Specifically, a template specifies a component's **metadata** and its **code**.

Once defined, a component template can be instantiated as account components, which can then be grouped to form the account's `Code`.

## Component code

The component template’s code defines a library of functions that operate on the specified storage layout.

## Component metadata

The component metadata describes the account component entirely: its name, description, version, and storage layout.

The storage layout can optionally define specific initial values for each of the slots. Alternatively, placeholders can be utilized to identify values that should be provided at the moment of instantiation.

### TOML specification

The component metadata can be defined using TOML. Below is an example specification:

```toml
name = "Fungible Faucet"
description = "This component displays valid values for component templates"
version = "1.0.0"
supported-types = ["FungibleFaucet"]

[[storage]]
name = "token_metadata"
description = "Contains metadata about the token associated to the faucet account. The metadata is formed by three fields: max supply, the token symbol and the asset's decimals"
slot = 0
value = [
    { type = "felt", name = "max_supply", description = "Maximum supply of the token in base units" },
    { type = "tokensymbol", value = "TST" },
    { type = "u8", name = "decimals", description = "Number of decimal places for converting to absolute units", value = "10" },
    { value = "0x0" }
]

[[storage]]
name = "owner_public_key"
description = "This is a value placeholder"
slot = 1
type = "auth::rpo_falcon512::pub_key"

[[storage]]
name = "map_storage_entry"
slot = 2
values = [
    { key = "0x1", value = ["0x0", "249381274", "998123581", "124991023478"] },
    { key = "0xDE0B1140012A9FD912F18AD9EC85E40F4CB697AE", value = { name = "value_placeholder", description = "This value will be defined at the moment of instantiation", type = "word" } }
]
```

#### Header

The metadata header specifies four fields:

- `name`: The component template's name
- `description` (optional): A brief description of the component template and its functionality
- `version`: A semantic version of this component template
- `supported-types`: Specifies the types of accounts on which the component can be used. Valid values are `FungibleFaucet`, `NonFungibleFaucet`, `RegularAccountUpdatableCode` and `RegularAccountImmutableCode`

#### Storage entries

An account component template can have multiple storage entries. A storage entry can specify either a **single-slot value**, a **multi-slot value**, or a **storage map**.

Each of these storage entries contain the following fields:

- `name`: A name for identifying the storage entry
- `description` (optional): Describes the intended function of the storage slot within the component definition

Additionally, based on the type of the storage entry, there are specific fields that should be specified.

##### Single-slot value

A single-slot value fits within one slot (i.e., one word).

For a single-slot entry, the following fields are expected:

- `slot`: Specifies the slot index in which the value will be placed
- `value` (optional): Contains the initial storage value for this slot. Will be interpreted as a `word` unless another `type` is specified
- `type` (optional): Describes the expected type for the slot

If no `value` is provided, the entry acts as a placeholder, requiring a value to be passed at instantiation. In this case, specifying a `type` is mandatory to ensure the input is correctly parsed.
Valid types for a single-slot value are `word` or `auth::rpo_falcon512::pub_key`.

Note that at least one of `value` and `type` should be specified. 

In the above example, the first and second storage entries are single-slot values.

##### Multi-slot value

Multi-slot values are composite values that exceed the size of a single slot (i.e., more than one `word`).

For multi-slot values, the following fields are expected:

- `slots`: Specifies the list of slots that the value comprises
- `values`: Contains the initial storage value for the specified slots

Placeholders can currently not be defined for multi-slot values.

##### Storage maps

Storage maps consist of key-value pairs, where both keys and values are single words.

Storage map entries can specify the following fields:

- `slot`: Specifies the slot index in which the value will be placed
- `values`: Contains a list of map entries, defined by a `key` and `value `

Storage maps `key`s and `value`s are word-long values, which can be defined as placeholders.

In the example, the third storage entry defines a storage map.

#### Specifying values and their types

In the TOML format, any value that is one word long can be written as a single value, or as exactly four field elements. In turn, a field element is a number within Miden's finite field. 

A word can be written as a hexadecimal value, and field elements can be written either as hexadecimal or decimal numbers.

In our example, the `token_metadata` single-slot entry is defined as four elements, where the first element is a placeholder, and the second, third and fourth are hardcoded values.

##### Word-long types

Valid word-long types are `word` (default type) and `auth::rpo_falcon512::pub_key` (represents a Falcon public key). Both can be parsed as hexadecimal strings.

##### Felt-long types

Valid field element types are `u8`, `u16`, `u32`, `felt` and `tokensymbol`:

- `u8`, `u16` and `u32` values can be parsed as decimal numbers and represent 8-bit, 16-bit and 32-bit values
- `felt` values represent a field element, and can be parsed as decimal or hexadecimal values
- `tokensymbol` values represent the symbol for basic fungible tokens, and are parsed as strings made of four uppercase characters
