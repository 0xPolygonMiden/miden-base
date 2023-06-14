# Overview
This documentation presents detailed guides on:

* Polygon Miden [Architecture](https://0xpolygonmiden.github.io/miden-base/architecture.html)
* Polygon Miden [Network Design](https://0xpolygonmiden.github.io/miden-base/network.html) [WIP]
* Participating in the [Polygon Miden Testnet](https://0xpolygonmiden.github.io/miden-base/developer-info.html) [WIP]
* Polygon Miden's [Cryptographic Primitives](https://0xpolygonmiden.github.io/miden-base/crypto-primitives.html)

The Polygon Miden **Architecture** describes Miden's unique state and execution model - an actor-based model with concurrent off-chain state. 

From the **Network Design** perspective, Polygon Miden uses a bi-directional token bridge and verifier contract to ensure computational integrity [WIP]. Miden Nodes act as operators that keep the state and compress state transitions recursively into STARK-proofs. The token bridge on Ethereum verifies these proofs. Users can run Miden Clients to send RPC requests to the Miden Nodes to update the state.

Once we are ready, we will provide an in-depth guide on how developers can participate to use our **testnet**. 

# Goal: Extend Ethereum’s feature set  
With Polygon Miden, we aim to extend Ethereum's feature set. Ethereum is designed to be a base layer that evolves slowly and provides stability. Rollups allow the creation of new design spaces while retaining the security of Ethereum. This makes a rollup the perfect place to innovate and enable new functionality.

Unlike many other rollups, Polygon Miden prioritizes ZK-friendliness over EVM compatibility. It also uses a novel state model to exploit the full power of a ZK-centric design. These design decisions allow developers to create applications that are currently difficult or impractical to build on account-based systems. 

We extend Ethereum on three core dimensions to attract billions of users: scalability, safety, and privacy.

## Scalability
To achieve ultimate scalability, we radically change how blockchains are designed. Polygon Miden changes the paradigm that everything in a blockchain must be transparent to be verifiable. 

Blockchains verify by re-executing. Re-executing requires transparency and processing power. Verification by re-execution slows blockchains down. Zero-knowledge proofs offer the possibility to verify without re-execution. Zero-knowledge verification doesn’t need transparency or processing power. In Polygon Miden, users can generate their own proofs, and the network verifies them.

This is the most important change in Polygon Miden. Users can execute smart contracts locally. Specifically, for anything that doesn’t touch the public state, users can execute smart contracts on their devices and then send ZK proofs to the network. The operators can then verify these ZK proofs exponentially faster than executing the original transactions and update the state accordingly. 

Not only does this reduce the computational burden on the operators, but it also makes such transactions inherently parallelizable. Even more exciting is that it lifts the limits on what can go into a smart contract. For example, anything that a user can execute and prove locally - no matter how complex - can be processed by the network with minimal costs. On Miden, it will be cheap to run even complex computations.

Another important change in Polygon Miden is ensuring that most transactions do not need to touch the public state. We achieve this by making all interactions between smart contracts asynchronous. With Polygon Miden, token transfers, NFT swaps, and many others do not need to touch the public state. For actions that change the public state, Polygon Miden does allow regular network execution of transactions (same as most other blockchains). Still, because of the asynchronous execution model, interactions between locally executed transactions and network transactions are done seamlessly.

## Safety
Assets need to be safe and easy to handle. No one should lose their tokens when losing a key or sending them to the wrong address. Polygon Miden’s approach aims to reduce the risks of using crypto on multiple fronts.

First, every account on Polygon Miden is a smart contract. This is frequently referred to as account abstraction. This enables developers building on Polygon Miden to create safer user wallets with features like social recovery of keys, rate-limiting spending tokens, transaction risk analysis, etc.

Next, because of Polygon Miden’s asynchronous execution model, it is possible to create recallable transactions, which mitigate the risk of sending funds to a non-existent address. This provides a safer environment for users.

Another change that increases safety is that in Miden, fungible and non-fungible assets are stored locally in accounts (rather than in global token contracts). This makes exploiting potential bugs more difficult, as every account needs to be attacked individually.

Speaking of bugs, to make smart contract development safer, Polygon Miden aims to support modern smart contract languages such as Move and Sway. These languages were designed with an emphasis on safety and correctness and incorporated years of experience and features from other safe languages, such as Rust, in their design.

## Privacy
Lastly, absolute transparency is one of the main drawbacks of blockchains. The ability to transact in private is a fundamental right and a practical necessity. And thus, we put privacy at the core of Polygon Miden’s design. 

But we go beyond simple private transactions: Polygon Miden’s architecture enables expressive private smart contracts. These are almost exactly the same as regular smart contracts but are executed locally so that the user does not reveal its code, state, and interaction graph to the network. And the coolest part is that private smart contracts can interact seamlessly with public smart contracts. So, for example, private rate-limited wallets can make calls to public DEXs. Businesses and financial institutions can build and execute their business logic on Miden. They would keep information hidden from competitors but visible to auditors.

Another important point regarding privacy is that users should not have to pay extra for it. In Polygon Miden’s design, private smart contracts impose minimal burden on the network (much smaller than public smart contracts), so on Polygon Miden, it is cheaper to remain private.

We understand that privacy is a complex area in the public domain. Privacy is a complex subject requiring careful study and consideration. We plan to enable privacy on Polygon Miden in stages. Initially, users can maintain privacy from other users but not from the operators (similar to Web2 privacy). This will give us time to figure out how to enable stronger levels of privacy without opening floodgates to potential abuses.
