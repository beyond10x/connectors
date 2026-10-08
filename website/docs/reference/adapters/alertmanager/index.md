---
title: Alertmanager
sidebar_position: 11
description: A design for alert-record reads; no native contract, model or runtime is published.
---

# Alertmanager

Alert records with explicit native configuration.

The adapter design describes intended alert-record reads and configuration. No standalone native
contract or typed model is published for this adapter yet. A future implementation must select
and validate its own profiles before advertising operations. No Alertmanager runtime is
implemented.
