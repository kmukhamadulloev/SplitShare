# Network Model

## Default listener

Default port:

```text
8080
```

The bind address is host-configurable.

Release defaults should favor safety:

- localhost-only before LAN sharing is enabled, or
- an explicit first-run sharing choice.

Do not silently expose all interfaces without making the state clear.

## Candidate addresses

A host may have:

- loopback;
- Ethernet;
- Wi-Fi;
- hotspot;
- Docker;
- VM bridge;
- WireGuard;
- Tailscale;
- other VPNs.

The network service returns typed candidates. The UI may rank:

1. physical/private LAN/hotspot;
2. explicit user-selected VPN;
3. other valid private adapters;
4. virtual bridges.

Never assume the first non-loopback address is the correct QR address.

## QR

QR generation is fully local.

No external QR image/API service.

The QR payload should use the active share-session join URL:

```text
http://192.168.1.52:8080/j/<token>
```

## Hotspot

Internet is not required.

A phone/laptop hotspot LAN works as long as the host listener is reachable from connected clients.

## IPv6

IPv6 may be supported after explicit tests for:

- URL formatting with brackets;
- interface scope;
- link-local scope IDs;
- QR payloads;
- access policy.

Do not advertise broken/unreachable IPv6 URLs.

## Public internet

SplitShare v1 is not an internet-facing server product.

Do not add UPnP port forwarding, public tunnels or remote relay as incidental features.

## Implemented Phase 05 selection

The network service returns all listener-compatible IPv4 candidates with adapter
name, address, classification and share URL through a host-only endpoint. The UI
allows explicit selection and locally generates a QR code. Name-based VPN/virtual
ranking is a heuristic; physical reachability is not inferred. Loopback remains
available with a host-only notice. Interface enumeration is a startup snapshot.
See [SESSIONS.md](SESSIONS.md).
