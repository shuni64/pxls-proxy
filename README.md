# Pxls IPv6 Proxy

<sub>(better name pending)</sub>

## About the project

This is a reverse proxy that redirects incoming requests to a given upstream through a given IPv6 subnet.
The outgoing local IP address is bound according to a hash of the incoming remote IP address.
This allows all clients to have a stable, unique outgoing IP address within the IPv6 prefix.
The hash is truncated to match the prefix, so the prefix must be large enough to avoid hash collisions.

No guarantees of stability are made, breaking changes may be made at any time.

## Getting started

### Prerequisites

- Linux
- Rust & Cargo (latest stable)
- OpenSSL

### Build

```bash
cargo build --release
```

### Usage
You first need a local Any-IP route for a sufficiently large IPv6 prefix.

```bash
ip route add local 2001:db8::/32 dev lo

```

You can then run the proxy with your prefix and a target upstream to connect to.

> [!IMPORTANT]
> The proxy trusts the `X-Forwarded-For` header by default.
> You very likely want to run another reverse proxy in front of this.

```bash
pxls-proxy --src-prefix 2001:db8::/32 --target-name pxls.space --target-port 443 --target-tls true
```
