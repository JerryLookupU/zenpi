# Subscription And Event Replay Gates

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

Fresh isolated Rust protocol gates passed:

```text
remote_subscription:     11 passed
remote_subscription_tls: 37 passed
remote_events:            6 passed
```

These cover Android-facing replay cursors, bounded subscriptions, transient
projection updates, event ACK behavior, rustls framing and TLS subscription
ownership. They remain host-side test evidence, not a connected Android or
physical-LAN receipt.
