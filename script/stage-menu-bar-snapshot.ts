#!/usr/bin/env bun
/** Stage the explicitly approved private GPUI fork; never publish crates. */
import { cpSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const version = "0.3.9-rust-hub.1";
const zedUrl = "https://github.com/humandane/zed";
const zedRev = "b8d1b870b359425ac7cbb51545008febeb4535fd";
const snapshotUrl = "https://github.com/humandane/gpui-menu-bar-snapshot";
const [zedArgument, destinationArgument] = process.argv.slice(2);
if (!zedArgument || !destinationArgument || process.argv.length !== 4) {
  throw new Error("usage: bun script/stage-menu-bar-snapshot.ts ZED_CHECKOUT NEW_DESTINATION");
}
const zed = resolve(zedArgument);
const destination = resolve(destinationArgument);
if (existsSync(destination)) throw new Error(`destination already exists: ${destination}`);

function run(command: string[], cwd: string, capture = false): string {
  const result = Bun.spawnSync(command, {
    cwd,
    stdout: capture ? "pipe" : "inherit",
    stderr: "inherit",
    env: process.env,
  });
  if (result.exitCode !== 0) throw new Error(`command failed: ${command.join(" ")}`);
  return capture ? result.stdout.toString().trim() : "";
}

if (run(["git", "rev-parse", "HEAD"], zed, true) !== zedRev) {
  throw new Error(`expected the reviewed Zed fork at ${zedRev}`);
}
if (run(["git", "status", "--porcelain"], zed, true)) {
  throw new Error("the Zed checkout must be clean");
}
run([process.execPath, "script/bump-gpui.ts", version, "--zed", zed, "--stage-only"], root);
const staging = join(root, "target/gpui-pre/workspace");
cpSync(staging, destination, {
  recursive: true,
  filter: path => !["target", ".git"].includes(path.split("/").at(-1) ?? ""),
});
const summary = JSON.parse(readFileSync(join(root, "target/gpui-pre/gpui-pre.json"), "utf8"));
summary.zed.url = zedUrl;
summary.repository = snapshotUrl;
summary.published = false;
summary.generator = {
  repository: "https://github.com/humandane/gpui-kit",
  base_rev: "0bbb5799ee8bb92fe448af300aa991c009b92be5",
  script: "script/bump-gpui.ts",
  sha256: createHash("sha256").update(readFileSync(join(root, "script/bump-gpui.ts"))).digest("hex"),
};
// These are exactly the registry requirements in the normalized manifests of
// the published gpui-pre 0.3.9 family. Git consumption otherwise retains the
// source fields that Cargo removes during crates.io publication.
const registryVersions = new Map([
  ["zed-font-kit", "0.14.1-zed"],
  ["oo7", "0.6"],
  ["zed-scap", "0.0.8-zed"],
  ["wasm_thread", "0.3"],
  ["zed-xim", "0.4.0-zed"],
]);
const normalized = new Map<string, object>();
function normalizeTable(table: Record<string, any> | undefined) {
  for (const [alias, spec] of Object.entries(table ?? {})) {
    if (typeof spec !== "object" || spec === null || !spec.git) continue;
    const name = spec.package ?? alias;
    if (registryVersions.get(name) !== spec.version) {
      throw new Error(`unreviewed external git dependency: ${name}`);
    }
    normalized.set(name, { package: name, version: spec.version, git: spec.git, rev: spec.rev });
    delete spec.git;
    delete spec.rev;
  }
}
const manifests = summary.crates.map((crate: { path: string }) => {
  const path = join(destination, crate.path, "Cargo.toml");
  return { path, manifest: Bun.TOML.parse(readFileSync(path, "utf8")) as any };
});
const inherited = new Set<string>();
for (const { manifest } of manifests) {
  const tables = [manifest.dependencies, manifest["build-dependencies"],
    ...Object.values(manifest.target ?? {}).flatMap((target: any) =>
      [target.dependencies, target["build-dependencies"]])];
  for (const table of tables) {
    for (const [alias, spec] of Object.entries(table ?? {})) {
      if ((spec as any)?.workspace) inherited.add(alias);
    }
    normalizeTable(table);
  }
}
const workspacePath = join(destination, "Cargo.toml");
const workspace: any = Bun.TOML.parse(readFileSync(workspacePath, "utf8"));
workspace.workspace.dependencies = Object.fromEntries(
  Object.entries(workspace.workspace.dependencies).filter(([alias]) => inherited.has(alias)),
);
normalizeTable(workspace.workspace.dependencies);
workspace.workspace.package.publish = false;
writeFileSync(workspacePath, Bun.TOML.stringify(workspace));
for (const { path, manifest } of manifests) {
  manifest.package.publish = false;
  manifest.package.repository = snapshotUrl;
  manifest.package.metadata["gpui-pre"]["zed-repository"] = zedUrl;
  writeFileSync(path, Bun.TOML.stringify(manifest));
}
summary.registry_normalization = [...normalized.values()];
writeFileSync(join(destination, "gpui-pre.json"), JSON.stringify(summary, null, 2) + "\n");
writeFileSync(join(destination, ".gitignore"), "/target/\n.DS_Store\n");
writeFileSync(join(destination, "README.md"), `# Private GPUI menu bar snapshot

This is an unpublished development fork for rust-hub, version \`${version}\`.
It is not an official GPUI or gpui-pre release. Every package has
\`publish = false\`; consumers pin the repository's full Git commit.

Source: [humandane/zed](${zedUrl}/tree/${zedRev}) at \`${zedRev}\`.
The source adds per-window hiding and unfiltered native reopen requests.
The generated crate family preserves the Kit facade macro transformations,
declarative macro paths, Apple shared sources, and WGPU publication adaptation.
The original Apache-2.0 licenses and notices are retained in every crate.

Reproduce from the matching GPUI Kit fork with Bun 1.4.4 and Rust 1.99.0:

\`\`\`sh
RUSTUP_TOOLCHAIN=1.99.0 bun script/stage-menu-bar-snapshot.ts /path/to/zed /new/output
\`\`\`

The staging wrapper checks the exact clean Zed source, invokes upstream's
\`--stage-only\` generator, records fork provenance, removes unused workspace
dependencies, and normalizes five versioned external Git dependencies to the
registry requirements used by published gpui-pre 0.3.9. It never publishes.
See \`gpui-pre.json\` for source hashes, package inventory, generator provenance,
pruned features, and the previous external Git revisions.

Validation belongs to the consuming Kit and rust-hub forks. Native manual
checks are deferred; successful compilation does not establish native UI behavior.
Return to official aligned GPUI/Kit releases when these capabilities are available.
`);
const metadata = JSON.parse(run(["cargo", "metadata", "--format-version", "1"], destination, true));
const internal = metadata.packages.filter((pkg: any) => metadata.workspace_members.includes(pkg.id));
if (internal.length !== summary.crates.length || internal.some((pkg: any) =>
  pkg.version !== version || pkg.publish?.length !== 0 || pkg.license !== "Apache-2.0")) {
  throw new Error("generated packages are not aligned, unpublished Apache-2.0 crates");
}
if (metadata.packages.some((pkg: any) => pkg.source?.startsWith("git+"))) {
  throw new Error("registry normalization left a transitive Git source");
}
const copyleft = /\b(?:A?GPL|LGPL|SSPL|EUPL|OSL|CPAL|BUSL|CC-BY-SA)\b/i;
if (metadata.packages.some((pkg: any) => pkg.license && pkg.license
  .split(/\s+OR\s+|\//i).every((alternative: string) => copyleft.test(alternative)))) {
  throw new Error("normalized dependency graph requires a copyleft license");
}
console.log(`Private snapshot ready at ${destination}`);
