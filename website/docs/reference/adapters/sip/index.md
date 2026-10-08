---
title: SIP
sidebar_position: 12
description: A native SIP dial contract behind the shared session and media contracts; no SIP runtime.
---

# SIP

Protocol-specific dial behaviour behind shared session and media contracts.

The native dial profile separates the external effect of placing a call from its later session
and media lifetime. Negotiation, cancellation and uncertain outcomes need explicit semantics. SIP
protocol code stays independent of other media adapter implementations. The dial contract is
documented; no SIP runtime is implemented.

## Native contract reference

- [SIP dial](./contracts/dial.md)
