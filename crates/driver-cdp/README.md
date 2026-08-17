# driver-cdp

The closed built-in `cdp_v1` protocol driver: the only Connectors crate allowed to launch a browser
and speak the Chrome DevTools Protocol. It consumes a non-serializable `service::AdmittedBrowserPlan`
and never mints one.

## Why the driver is `cdp_v1` and the operations are `browser.*`

The `protocol_driver` axis answers *which closed implementation speaks to the external system*. The
external system is a browser, and the Chrome DevTools Protocol is how one is spoken to — which is
`src/cdp.rs`. Brave, Chrome and Chromium are three package names for one protocol, not three
drivers, so `src/chromium.rs` is one implementation of the neutral `src/page.rs` port.

| Later capability | New driver? | Where it lands |
|---|---|---|
| clicking, typing, submitting | no | a mutating operation over the same `page` port |
| reading a page's network log | no | a second observation over the same `cdp` transport |
| a headless browser in a container | no | the same driver at `placement_requirement = substrate_workload` |
| a Firefox / WebDriver BiDi browser | **yes** | a different protocol, and therefore a different driver word |

Naming the driver `browser_v1` would have forced a lie for the last row.

## The axis values, and the argument for each

| Axis | Value | Why not the neighbour |
|---|---|---|
| `interaction_shape` | `leased_session` | The process, dedicated profile and attached page survive **between** calls, and the element references one snapshot hands out are valid only until the next one. `unary` would deny that. `session_establishment` — what `sip.dial` uses — would promise a direct-byte plane and a short-lived endpoint authority; there is none, and every browser result returns through the ordinary bounded operation path. |
| `protocol_driver` | `cdp_v1` | See above. |
| `placement_requirement` | `connectors_deployment` | Unlike a speaker, a browser needs no person present, so `federated_satellite` would refuse a hosted deployment that can serve this perfectly well. `substrate_workload` is the form a container-isolated browser takes (architecture ADR 0035) and stays unclaimed until a driver actually asks Substrate to run one: claiming it now would demand a container runtime nothing here uses, and would silently drop the retained profile. |
| `implementation_form` | `built_in` | The only admitted form; attested out-of-process artifacts remain deferred. |
| `required_capabilities` | `public_network`, `process` | The driver spawns a browser, and that browser reaches the public internet. Declared uniformly on all five operations because the capability belongs to the **lease**, not to the individual call: a placement that could never admit `browser.open` must not be allowed to admit `browser.close` either, or it could end a session it could never have started. |

**On `process` as a required capability rather than only a host effect.** `driver-audio` declares
only `device` and carries `process` in `effects`, because `device` already implies a satellite with a
sound card and the process spawn is subordinate to it. A browser has no `device` to stand in for it.
`RequiredCapability` is what `service::plan_operation` refuses on — a deployment lacking the
authority never receives the plan — while `HostEffect` is descriptive and reaches the authority
projection. Leaving `process` out of `required_capabilities` would mean nothing in planning stops a
placement with no process authority from being handed a browser launch. So `cdp_v1` declares it in
both places, and the two say different things on purpose.

## The shipped surface

Five operations, declared in `providers/b10x.toml`: `browser.open`, `browser.goto`,
`browser.snapshot`, `browser.screenshot`, `browser.close`. All read-only, no approval.

**Interaction — clicking, typing, submitting — is deliberately absent.** It acts on someone else's
system on the operator's behalf, so it is a mutation, and it waits on the approval round-trip being
built separately. `catalog-build`'s `the_browser_surface_is_read_only_and_carries_no_interaction_member`
fails the whole catalogue if one appears.

## Two silent failure modes this crate exists to prevent

1. **A resolved executable path is never canonicalized.** `/usr/bin/brave` is a shell wrapper, and
   multi-call binaries elsewhere on the same machine select their behavior from `argv[0]`; executing
   the canonical target instead changes what the program does while still starting.
   `chromium::tests::a_wrapper_launcher_is_never_canonicalized_to_its_target` pins it.
2. **The launch flag is never trusted.** `/usr/bin/brave` `exec`s the real binary with `"$@"`
   followed by the operator's own `brave-flags.conf` entries — user flags are appended *after* ours —
   so a supplied `--remote-debugging-port` can be overridden. `/usr/bin/google-chrome-stable` is the
   same pattern. The port is requested as `0` and read back from `<user-data-dir>/DevToolsActivePort`,
   which is immune to flag ordering. `cdp::tests::a_zero_or_malformed_active_port_is_not_accepted`
   pins the read-back.

## Four properties held above the transport

1. **The profile is dedicated, never the operator's own.** A page this driver visits holds none of
   the operator's logged-in sessions, so a page that tries to steer the agent cannot act as them
   against their accounts. `service::validate_browser_deployment_route` refuses a route pointing at
   `~/.config/BraveSoftware`, `~/.config/google-chrome`, `~/.config/chromium` and their macOS and
   Windows spellings before this crate is reached.
2. **Only `http`/`https` addresses are admitted.** `file:`, `chrome:`, `devtools:`, `about:` and
   `javascript:` are refused: each would turn page reading into local file reading, privileged
   browser control, or script execution inside the profile. `page::PageAddress` cannot be
   constructed from one.
3. **Bounds are reported, not silently applied.** An oversized page returns `truncated` with both
   counts; a stale element reference refuses and asks for a fresh snapshot rather than acting on
   whatever occupies that position now.
4. **Page content travels inside an untrusted-content envelope.** `protocol::browser::PageView`
   carries the label as a **required** field, so page text cannot reach a model without it.

Pages reach a model as an accessibility tree, not as pixels: no image content block exists anywhere
in this stack, and a rendered page would exceed the operation-result bound many times over.
Screenshots are written to disk for the operator, and the model receives a path, a digest and a size.
Closing retains the dedicated profile directory, so a site the operator logged into once inside it
stays logged in for the next session.

## Dependencies

The driver's only HTTP call is `http://127.0.0.1:<port>/json/list`, the DevTools target list.
`reqwest` is therefore taken with `default-features = false` and **no TLS backend**: the aperture is
loopback plaintext by construction rather than by policy.

## Tests

No test spawns a browser, opens a socket, or requires a window. Browser behavior is covered through a
fake engine implementing the same port. Two `#[ignore]`d live checks exercise a real local browser
and are run deliberately:

```bash
cargo test --manifest-path crates/driver-cdp/Cargo.toml -- --ignored --nocapture
```

The crate is an intentionally nested workspace with its own lock, like `driver-sip` and
`driver-audio`: process- and socket-capable code stays out of the deterministic catalogue-compiler
workspace.
