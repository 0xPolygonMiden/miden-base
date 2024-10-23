# Proxy server

Server to proxy calls to the proving service. It is built using Cloudflare's Pingora crate
which provides proxy, load balancing, rate limiting, timeout and gRPC compatibility.

Further information about Pingora can be [found here](https://github.com/cloudflare/pingora).

## Overview

For proxy, rate limiting, timeout and gRPC support we plan to use Pingoras feature almost
out-of-the-box, which requires little to none configuration.

For load balancing we are implementing our own strategy since we want to support a queue, only
hitting the upstream server with 1 request at a time. Additionally,
[Pingora's CTX](https://github.com/cloudflare/pingora/blob/main/docs/user_guide/ctx.md) is going to
be helpful to implement the shared queues.

More information and discussion about the implementation
[available here](https://github.com/0xPolygonMiden/miden-base/issues/908).

## Usage

To build the prover proxy, from the root of the workspace you can run:

```bash
make install-prover-proxy
```

And then you can run it by doing:

```bash
miden-prover-proxy
```
