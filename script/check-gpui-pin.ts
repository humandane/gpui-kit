#!/usr/bin/env bun
/**
 * Verify that the workspace pins every `gpui-pre-*` crate to an exact version.
 *
 * The published gpui-kit crates carry the workspace's requirements to
 * crates.io. A caret requirement lets the weekly gpui-pre release move an
 * application onto a snapshot that gpui-component was never built against,
 * which is how gpui-kit 0.6.4 stopped compiling the day gpui-pre 0.3.6 came
 * out (#3156). Every snapshot crate has to stay on one `=x.y.z` requirement,
 * and all of them on the same version, so a release resolves exactly the
 * snapshot it was tested with. The hand-published `gpui-pre-reqwest` fork
 * (see `DEPENDENCY_OVERRIDES` in bump-gpui.ts) has its own version line and
 * only needs to be exact.
 *
 *     bun script/check-gpui-pin.ts [path/to/Cargo.toml]
 */
import { readFileSync } from "node:fs";
import { strict as assert } from "node:assert";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const MANIFEST = "Cargo.toml";
const SNAPSHOT_PREFIX = "gpui-pre";
const OWN_VERSION_LINE = new Set(["gpui-pre-reqwest"]);
const EXACT_VERSION = /^=(\d+\.\d+\.\d+)$/;
// Explicit, temporary development exception. Official snapshots remain registry
// releases; this private family is never published and must use one full SHA.
const DEVELOPMENT_VERSION = "=0.3.9-rust-hub.1";
const DEVELOPMENT_SOURCE = "https://github.com/humandane/gpui-menu-bar-snapshot";
const FULL_REVISION = /^[0-9a-f]{40}$/;

interface Dependency {
  alias: string;
  package: string;
  version: string | undefined;
  git?: string;
  rev?: string;
  branch?: string;
  tag?: string;
  path?: string;
  registry?: string;
}

function snapshotDependencies(manifestPath: string): Dependency[] {
  const manifest = Bun.TOML.parse(readFileSync(manifestPath, "utf8")) as {
    workspace?: { dependencies?: Record<string, unknown> };
  };
  const dependencies: Dependency[] = [];
  for (const [alias, spec] of Object.entries(manifest.workspace?.dependencies ?? {})) {
    if (typeof spec !== "object" || spec === null) continue;
    const { package: pkg, version, ...source } = spec as Record<string, unknown>;
    if (typeof pkg !== "string" || !pkg.startsWith(SNAPSHOT_PREFIX)) continue;
    dependencies.push({ alias, package: pkg, version: typeof version === "string" ? version : undefined,
      ...Object.fromEntries(["git", "rev", "branch", "tag", "path", "registry"]
        .filter(key => source[key] !== undefined).map(key => [key, source[key]])) });
  }
  return dependencies;
}

function describe(dependency: Dependency): string {
  return `${dependency.alias} = { package = "${dependency.package}", version = ${JSON.stringify(dependency.version ?? "")} }`;
}

function problems(dependencies: Dependency[]): string[] {
  const found: string[] = [];
  if (dependencies.length === 0) found.push(`no \`${SNAPSHOT_PREFIX}-*\` dependency in [workspace.dependencies]`);
  const snapshotVersions = new Map<string, string[]>();
  const development = dependencies.some(dependency => dependency.version === DEVELOPMENT_VERSION);
  const revisions = new Set<string>();
  for (const dependency of dependencies) {
    const ownVersion = OWN_VERSION_LINE.has(dependency.package);
    if (development && !ownVersion) {
      if (dependency.version !== DEVELOPMENT_VERSION || dependency.git !== DEVELOPMENT_SOURCE ||
          typeof dependency.rev !== "string" || !FULL_REVISION.test(dependency.rev) ||
          dependency.branch !== undefined || dependency.tag !== undefined ||
          dependency.path !== undefined || dependency.registry !== undefined) {
        found.push(`${dependency.package} must use ${DEVELOPMENT_VERSION} from ${DEVELOPMENT_SOURCE} at one full revision`);
      } else {
        revisions.add(dependency.rev);
      }
      continue;
    }
    if ([dependency.git, dependency.rev, dependency.branch, dependency.tag,
      dependency.path, dependency.registry].some(value => value !== undefined)) {
      found.push(`${dependency.package} must use its official registry source`);
    }
    const exact = dependency.version === undefined ? null : EXACT_VERSION.exec(dependency.version);
    if (exact === null) {
      found.push(`${describe(dependency)} is not an exact \`=x.y.z\` requirement`);
      continue;
    }
    if (ownVersion) continue;
    const aliases = snapshotVersions.get(exact[1]) ?? [];
    aliases.push(dependency.package);
    snapshotVersions.set(exact[1], aliases);
  }
  if (revisions.size > 1) found.push("the development snapshot crates use different Git revisions");
  if (snapshotVersions.size > 1) {
    const detail = [...snapshotVersions].map(([version, names]) => `${version} (${names.join(", ")})`).join(", ");
    found.push(`the snapshot crates are pinned to more than one version: ${detail}`);
  }
  return found;
}

if (process.argv[2] === "--self-test") {
  const core = { alias: "gpui", package: "gpui-pre", version: "=0.3.9" };
  const platform = { ...core, alias: "gpui_platform", package: "gpui-pre-platform" };
  const reqwest = { ...core, alias: "reqwest", package: "gpui-pre-reqwest", version: "=0.12.15" };
  const fork = { ...core, version: DEVELOPMENT_VERSION, git: DEVELOPMENT_SOURCE, rev: "a".repeat(40) };
  assert.deepEqual(problems([core, platform, reqwest]), []);
  assert.notDeepEqual(problems([{ ...core, version: "0.3.9" }]), []);
  assert.notDeepEqual(problems([core, { ...platform, version: "=0.3.8" }]), []);
  assert.deepEqual(problems([fork, { ...fork, package: platform.package }, reqwest]), []);
  for (const invalid of [
    { ...fork, rev: "main" }, { ...fork, rev: undefined },
    { ...fork, git: "https://example.com/unreviewed" },
    { ...fork, branch: "main" }, { ...fork, tag: "v0.3.9" },
    { ...fork, path: "../snapshot" }, { ...fork, registry: "private" },
  ]) assert.notDeepEqual(problems([invalid]), []);
  assert.notDeepEqual(problems([fork, platform]), []);
  assert.notDeepEqual(problems([fork, { ...fork, package: platform.package, rev: "b".repeat(40) }]), []);
  assert.notDeepEqual(problems([{ ...core, git: DEVELOPMENT_SOURCE, rev: "a".repeat(40) }]), []);
  assert.notDeepEqual(problems([fork, { ...reqwest, git: DEVELOPMENT_SOURCE }]), []);
  assert.notDeepEqual(problems([]), []);
  console.log("GPUI pin checks: 17 valid/invalid graph cases passed");
  process.exit(0);
}

const manifestPath = process.argv[2] ?? join(REPO_ROOT, MANIFEST);
const found = problems(snapshotDependencies(manifestPath));
if (found.length > 0) {
  for (const problem of found) console.error(`::error file=${MANIFEST}::${problem}`);
  console.error(`\n${MANIFEST} must pin every ${SNAPSHOT_PREFIX}-* crate to the exact snapshot gpui-kit was built against.`);
  process.exit(1);
}
console.log(`${MANIFEST} pins the ${SNAPSHOT_PREFIX}-* crates exactly`);
