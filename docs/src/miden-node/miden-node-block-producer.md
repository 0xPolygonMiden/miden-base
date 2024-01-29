# Miden block producer
The **Block producer** receives transactions from the RPC component, processes them, creates block containing those transactions before sending created blocks to the store. 

**Block Producer** is one of components of the Miden node. 

## Architecture
`TODO`

## API
The **Block Producer** serves connections using the [gRPC protocol](https://grpc.io) on a port, set in the previously mentioned configuration file. The API cannot directly be called by the Miden Client. 

Here is a brief description of supported methods.

### SubmitProvenTransaction

Submits proven transaction to the Miden network.

**Parameters**

* `transaction`: `bytes` - transaction encoded using Miden's native format.

**Returns**

This method doesn't return any data.