# Private menu bar development graph

This branch temporarily consumes an unpublished GPUI fork so rust-hub can use
the native status item and login integration together with retained main-window
identity. The user explicitly approved this exception to the normal published
snapshot policy. It is not an official GPUI Kit or gpui-pre release.

Applications still depend only on `gpui-kit`, pinned to this fork's full Git
revision. No application `[patch]`, direct GPUI dependency, or crate alias is
required. Keep `gpui-fast`, `speech`, and GPUI screen capture disabled for this
consumer. Fast has its own dependency graph and is not the validated backend.

The six snapshot dependencies in `Cargo.toml` all use
`gpui-pre 0.3.9-rust-hub.1` from
<https://github.com/humandane/gpui-menu-bar-snapshot> at
`2d70a2a015b7613246efb59554a141d87fdb70d7`.
Its 24 internal packages are generated from
<https://github.com/humandane/zed> at
`b8d1b870b359425ac7cbb51545008febeb4535fd` and have `publish = false`.
The separately published `gpui-pre-reqwest =0.12.15` stays on its own version
line. The snapshot normalizes versioned external Git dependencies to the
registry requirements used by the published 0.3.9 package manifests; its
resolved dependency graph requires no additional Git source allowances.

The snapshot retains Kit's generated proc-macro facade rewriting, crate-relative
`actions!` derive paths, Apple shared-source packaging, and WGPU 30 publication
adaptation. Pointing the six dependencies directly at raw Zed source would lose
these packaging transformations. The private fork adds `Window::hide_window`
and `Application::on_reopen_request`; their native behavior was developed and
tested in the separate Zed fork.

The newer upstream source also makes modifier checks and keystroke formatting
depend on the host OS, restricts context-free keystroke parsing to static text
with explicit platform modifiers,
and renames scroll input to `ScrollEvent` / `on_scroll`. Kit's private input
adapters and test helpers use the new APIs with the current App context; the
optional Fast branches keep their existing API calls. No application-facing
Kit helper signature changes. Built-in `secondary-` bindings use
`KeyBinding::load` with the current App, including the status example's Quit
binding. `KeyBinding::new` no longer accepts that platform-neutral modifier.

## Reproduction and validation

With Bun 1.4.4, Rust 1.99.0, and a clean checkout of the exact Zed fork revision:

```sh
RUSTUP_TOOLCHAIN=1.99.0 bun script/stage-menu-bar-snapshot.ts /path/to/zed /new/snapshot
bun script/check-gpui-pin.ts --self-test
bun script/check-gpui-pin.ts
cargo +1.99.0 check --locked -p gpui-kit -p status_item
cargo +1.99.0 test --locked --no-default-features --features test-support --lib -p gpui-kit
cargo +1.99.0 test --locked --features test-support --test ui -p gpui-kit
cargo +1.99.0 build --locked -p status_item
```

The staging command never publishes. It validates source identity, runs the
existing upstream generator with `--stage-only`, records fork provenance,
retains licenses, normalizes the reviewed registry dependencies, and verifies
the final package/source/license invariants. `gpui-pre.json` in the snapshot
records the source revision, original generator hash, package inventory, and
external dependency transformations.

The pin checker permits only this prerelease and repository as the development
exception. It requires one full revision across the family and rejects branches,
tags, path overrides, registry substitutions, or a mixture with official
snapshots. Its self-tests cover accepted and rejected source graphs.

The Linux workflow runs the Kit library tests, the profile UI smoke test, and
the status example check on Ubuntu 24.04. Native manual checks of the combined
graph are deferred. Record
automated results separately; a successful compile is not proof of native focus,
accessibility, login-item behavior, or popup dismissal.

Consumers must synchronize vendored skills from the locked Kit fork revision
and record its repository and SHA, rather than inferring provenance from the
unchanged `0.8.0` package version. Return to official aligned snapshots and a
published Kit release when the required capabilities are available upstream.
