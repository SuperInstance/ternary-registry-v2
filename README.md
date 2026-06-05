# ternary-registry-v2: Enhanced skill registry with versioning and dependencies

An upgrade over `ternary-registry` that adds semantic versioning, dependency constraints, conflict detection, compatibility matrices, and cross-room registry syncing.

## Why This Exists

The original `ternary-registry` tracked skills by tier (Basic/Standard/Advanced/Expert) but had no concept of versions, dependencies, or conflicts. In practice, skills depend on each other, evolve independently, and sometimes can't coexist. RegistryV2 adds the machinery to model all of that: semver versioning, required and optional dependencies with min/max version constraints, explicit conflict declarations, and the ability to compare registries across rooms.

## Core Concepts

- **RegistryV2**: The main registry. Register skills with versions and dependencies, detect conflicts, check dependency satisfaction.
- **SkillVersion**: Semantic version (major.minor.patch). Versions with the same major are "compatible."
- **SkillDep**: A dependency on another skill, with optional min/max version bounds and optional/required flag.
- **SkillConflict**: An explicit declaration that two skills can't coexist, with a human-readable reason.
- **SkillCompat**: A symmetric matrix tracking which skill pairs are known compatible or incompatible.
- **RegistrySync**: Compares registries across rooms — finds common skills, unique skills, and version mismatches.

## Quick Start

```toml
[dependencies]
ternary-registry-v2 = "0.1"
```

```rust
use ternary_registry_v2::*;

let mut reg = RegistryV2::new();
reg.register("base-skill", SkillVersion::new(1, 0, 0), vec![]);
reg.register("advanced-skill", SkillVersion::new(2, 1, 0), vec![
    SkillDep::required("base-skill").with_min_version(SkillVersion::new(1, 0, 0)),
]);

assert!(reg.check_dependencies("advanced-skill").is_ok());
assert_eq!(reg.version_of("base-skill"), Some(SkillVersion::parse("1.0.0").unwrap()));
```

## API Overview

| Type | Description |
|------|-------------|
| `RegistryV2` | Central registry: register, deprecate, check deps, detect conflicts |
| `SkillVersion` | Semver (major.minor.patch) with compatibility and comparison |
| `SkillDep` | Dependency declaration with version range and optional flag |
| `SkillConflict` | Named pair of skills that can't coexist |
| `SkillCompat` | Symmetric compatibility matrix for skill pairs |
| `RegistrySync` | Multi-room registry comparison (common/unique/mismatched skills) |

## How It Works

`RegistryV2` stores skills in a `HashMap<String, RegistryEntry>` where each entry tracks version, dependencies, and deprecation status. Dependency checking walks the dependency list and verifies each required dep is registered, not deprecated, and within version bounds.

Conflicts are stored as a flat `Vec<SkillConflict>` and checked via linear scan. This is O(n) per conflict check — fine for registries with dozens of skills, less ideal for thousands.

`SkillCompat` is a `HashMap<(String, String), bool>` stored symmetrically (both directions). Absent pairs are assumed compatible (open-world assumption).

`RegistrySync` takes multiple `RegistryV2` instances and computes set intersections and differences. It finds common skills, skills unique to one room, and version mismatches for shared skills.

## Known Limitations

- **No dependency resolution**: `check_dependencies` verifies direct deps only. Transitive dependency chains aren't automatically resolved.
- **No circular dependency detection**: If A depends on B and B depends on A, both pass individual checks.
- **Linear conflict scanning**: Large conflict lists mean slower conflict checks. Consider indexing if you have hundreds of conflicts.
- **No version ranges beyond min/max**: Can't express "1.x but not 1.3" or tilde/caret ranges.
- **Sync doesn't merge**: `RegistrySync` compares registries but doesn't produce a merged result.

## Use Cases

- **Skill rollout**: Before deploying a new skill, check it doesn't conflict with existing ones and all dependencies are met.
- **Multi-room consistency**: Use `RegistrySync` to find rooms running different skill versions that should be aligned.
- **Deprecation migration**: Mark old skills as deprecated and track which rooms still use them.
- **Compatibility testing**: Build a `SkillCompat` matrix from integration test results and query it during planning.

## Ecosystem Context

Upgrades `ternary-registry` (which provides basic skill tracking by tier). Related crates:
- `ternary-room`: Rooms host registries.
- `ternary-protocol`: How registries sync across rooms at the protocol level.

## License

MIT

## See Also
- **ternary-registry** — related
- **ternary-database** — related
- **ternary-archive** — related
- **ternary-beacon** — related
- **ternary-protocol** — related

