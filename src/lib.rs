#![forbid(unsafe_code)]

//! Enhanced skill registry with versioning and dependency management.
//!
//! Upgrades `ternary-registry` with semantic versioning, dependency declarations,
//! conflict detection, compatibility matrices, and cross-room registry syncing.
//! Tracks which skills exist, what version they're at, what they depend on,
//! and whether they can coexist.

use std::collections::HashMap;
use std::collections::HashSet;

/// Semantic version: major.minor.patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkillVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SkillVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse from "major.minor.patch" string.
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(SkillVersion {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    /// Is this version compatible with another (same major version)?
    pub fn is_compatible_with(&self, other: &SkillVersion) -> bool {
        self.major == other.major
    }

    /// Is this a newer version than another?
    pub fn is_newer_than(&self, other: &SkillVersion) -> bool {
        (self.major, self.minor, self.patch) > (other.major, other.minor, other.patch)
    }
}

impl std::fmt::Display for SkillVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A dependency declaration: another skill with optional version constraint.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillDep {
    pub skill_name: String,
    pub min_version: Option<SkillVersion>,
    pub max_version: Option<SkillVersion>,
    pub optional: bool,
}

impl SkillDep {
    pub fn required(name: &str) -> Self {
        Self {
            skill_name: name.to_string(),
            min_version: None,
            max_version: None,
            optional: false,
        }
    }

    pub fn optional(name: &str) -> Self {
        Self {
            skill_name: name.to_string(),
            min_version: None,
            max_version: None,
            optional: true,
        }
    }

    pub fn with_min_version(mut self, v: SkillVersion) -> Self {
        self.min_version = Some(v);
        self
    }

    pub fn with_max_version(mut self, v: SkillVersion) -> Self {
        self.max_version = Some(v);
        self
    }

    /// Does a given version satisfy this dependency?
    pub fn is_satisfied_by(&self, version: &SkillVersion) -> bool {
        if let Some(min) = &self.min_version {
            if !version.is_newer_than(min) && version != min {
                return false;
            }
        }
        if let Some(max) = &self.max_version {
            if version.is_newer_than(max) {
                return false;
            }
        }
        true
    }
}

/// A conflict between two skills.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillConflict {
    pub skill_a: String,
    pub skill_b: String,
    pub reason: String,
}

impl SkillConflict {
    pub fn new(a: &str, b: &str, reason: &str) -> Self {
        Self {
            skill_a: a.to_string(),
            skill_b: b.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Check if a given skill is involved in this conflict.
    pub fn involves(&self, skill_name: &str) -> bool {
        self.skill_a == skill_name || self.skill_b == skill_name
    }
}

/// Entry in the registry.
#[derive(Clone, Debug)]
struct RegistryEntry {
    version: SkillVersion,
    dependencies: Vec<SkillDep>,
    deprecated: bool,
}

/// The enhanced registry.
#[derive(Clone, Debug)]
pub struct RegistryV2 {
    entries: HashMap<String, RegistryEntry>,
    conflicts: Vec<SkillConflict>,
}

impl RegistryV2 {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Register a skill with version and dependencies.
    pub fn register(&mut self, name: &str, version: SkillVersion, deps: Vec<SkillDep>) {
        self.entries.insert(
            name.to_string(),
            RegistryEntry {
                version,
                dependencies: deps,
                deprecated: false,
            },
        );
    }

    /// Mark a skill as deprecated.
    pub fn deprecate(&mut self, name: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.deprecated = true;
            true
        } else {
            false
        }
    }

    /// Get the version of a registered skill.
    pub fn version_of(&self, name: &str) -> Option<SkillVersion> {
        self.entries.get(name).map(|e| e.version)
    }

    /// Check if a skill is deprecated.
    pub fn is_deprecated(&self, name: &str) -> bool {
        self.entries.get(name).map(|e| e.deprecated).unwrap_or(false)
    }

    /// List all registered skill names.
    pub fn skill_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Get dependencies of a skill.
    pub fn dependencies_of(&self, name: &str) -> Vec<&SkillDep> {
        self.entries
            .get(name)
            .map(|e| e.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Check if all required dependencies of a skill are satisfied.
    pub fn check_dependencies(&self, name: &str) -> Result<(), String> {
        let entry = self.entries.get(name).ok_or_else(|| format!("skill '{}' not found", name))?;
        for dep in &entry.dependencies {
            if dep.optional {
                continue;
            }
            let dep_entry = self.entries.get(&dep.skill_name)
                .ok_or_else(|| format!("dependency '{}' not registered", dep.skill_name))?;
            if dep_entry.deprecated {
                return Err(format!("dependency '{}' is deprecated", dep.skill_name));
            }
            if !dep.is_satisfied_by(&dep_entry.version) {
                return Err(format!(
                    "dependency '{}' version {} does not satisfy constraint",
                    dep.skill_name, dep_entry.version
                ));
            }
        }
        Ok(())
    }

    /// Declare a conflict between two skills.
    pub fn declare_conflict(&mut self, conflict: SkillConflict) {
        self.conflicts.push(conflict);
    }

    /// Find all conflicts involving a skill.
    pub fn conflicts_for(&self, name: &str) -> Vec<&SkillConflict> {
        self.conflicts.iter().filter(|c| c.involves(name)).collect()
    }

    /// Check if two skills can coexist.
    pub fn can_coexist(&self, a: &str, b: &str) -> bool {
        !self.conflicts.iter().any(|c| c.involves(a) && c.involves(b))
    }

    pub fn skill_count(&self) -> usize {
        self.entries.len()
    }

    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }
}

impl Default for RegistryV2 {
    fn default() -> Self {
        Self::new()
    }
}

/// Compatibility matrix: tracks which skill versions work together.
#[derive(Clone, Debug)]
pub struct SkillCompat {
    /// (skill_a, skill_b) → compatible or not.
    matrix: HashMap<(String, String), bool>,
}

impl SkillCompat {
    pub fn new() -> Self {
        Self {
            matrix: HashMap::new(),
        }
    }

    /// Mark two skills as compatible or incompatible.
    pub fn set(&mut self, a: &str, b: &str, compatible: bool) {
        self.matrix.insert((a.to_string(), b.to_string()), compatible);
        self.matrix.insert((b.to_string(), a.to_string()), compatible);
    }

    /// Check compatibility. Returns true if no entry exists (assumed compatible).
    pub fn is_compatible(&self, a: &str, b: &str) -> bool {
        self.matrix
            .get(&(a.to_string(), b.to_string()))
            .copied()
            .unwrap_or(true)
    }

    /// Find all skills incompatible with a given one.
    pub fn incompatible_with(&self, name: &str) -> Vec<&str> {
        self.matrix
            .iter()
            .filter(|((a, b), &compat)| !compat && (a == name || b == name))
            .map(|((a, b), _)| if a == name { b.as_str() } else { a.as_str() })
            .collect()
    }

    pub fn entry_count(&self) -> usize {
        self.matrix.len() / 2 // symmetric
    }
}

impl Default for SkillCompat {
    fn default() -> Self {
        Self::new()
    }
}

/// Syncs registries across rooms.
#[derive(Clone, Debug)]
pub struct RegistrySync {
    /// Local registry snapshots per room.
    rooms: HashMap<String, RegistryV2>,
}

impl RegistrySync {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    /// Register a room's registry.
    pub fn add_room(&mut self, room_name: &str, registry: RegistryV2) {
        self.rooms.insert(room_name.to_string(), registry);
    }

    /// Get a room's registry.
    pub fn room_registry(&self, room_name: &str) -> Option<&RegistryV2> {
        self.rooms.get(room_name)
    }

    /// Find skills that exist in ALL rooms.
    pub fn common_skills(&self) -> HashSet<String> {
        let mut iter = self.rooms.values();
        let first = match iter.next() {
            Some(r) => r,
            None => return HashSet::new(),
        };
        let mut common: HashSet<String> = first.entries.keys().cloned().collect();
        for registry in iter {
            let keys: HashSet<String> = registry.entries.keys().cloned().collect();
            common = common.intersection(&keys).cloned().collect();
        }
        common
    }

    /// Find skills that only exist in one room.
    pub fn unique_skills(&self) -> HashMap<String, Vec<String>> {
        let all_skills: HashSet<String> = self
            .rooms
            .values()
            .flat_map(|r| r.entries.keys().cloned())
            .collect();

        let mut result = HashMap::new();
        for skill in all_skills {
            let rooms_with: Vec<String> = self
                .rooms
                .iter()
                .filter(|(_, r)| r.entries.contains_key(&skill))
                .map(|(name, _)| name.clone())
                .collect();
            if rooms_with.len() == 1 {
                result.insert(rooms_with[0].clone(), vec![]);
                result.get_mut(&rooms_with[0]).unwrap().push(skill);
            }
        }
        result
    }

    /// Find version mismatches for common skills across rooms.
    pub fn version_mismatches(&self) -> Vec<(String, Vec<(String, SkillVersion)>)> {
        let common = self.common_skills();
        let mut mismatches = Vec::new();
        for skill in common {
            let versions: Vec<(String, SkillVersion)> = self
                .rooms
                .iter()
                .filter_map(|(room, reg)| {
                    reg.entries.get(&skill).map(|e| (room.clone(), e.version))
                })
                .collect();
            let first_ver = versions[0].1;
            if versions.iter().any(|(_, v)| *v != first_ver) {
                mismatches.push((skill, versions));
            }
        }
        mismatches
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }
}

impl Default for RegistrySync {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_version_parse() {
        let v = SkillVersion::parse("1.2.3").unwrap();
        assert_eq!(v, SkillVersion::new(1, 2, 3));
        assert!(SkillVersion::parse("invalid").is_none());
        assert!(SkillVersion::parse("1.2").is_none());
    }

    #[test]
    fn test_version_display() {
        let v = SkillVersion::new(2, 1, 0);
        assert_eq!(format!("{}", v), "2.1.0");
    }

    #[test]
    fn test_version_compatible() {
        let v1 = SkillVersion::new(1, 0, 0);
        let v2 = SkillVersion::new(1, 5, 3);
        let v3 = SkillVersion::new(2, 0, 0);
        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }

    #[test]
    fn test_version_newer() {
        let v1 = SkillVersion::new(1, 0, 0);
        let v2 = SkillVersion::new(1, 0, 1);
        let v3 = SkillVersion::new(2, 0, 0);
        assert!(v2.is_newer_than(&v1));
        assert!(v3.is_newer_than(&v1));
        assert!(!v1.is_newer_than(&v2));
    }

    #[test]
    fn test_dep_satisfied() {
        let dep = SkillDep::required("foo").with_min_version(SkillVersion::new(1, 0, 0));
        assert!(dep.is_satisfied_by(&SkillVersion::new(1, 5, 0)));
        assert!(dep.is_satisfied_by(&SkillVersion::new(1, 0, 0)));
        assert!(!dep.is_satisfied_by(&SkillVersion::new(0, 9, 9)));
    }

    #[test]
    fn test_dep_with_max() {
        let dep = SkillDep::required("bar")
            .with_min_version(SkillVersion::new(1, 0, 0))
            .with_max_version(SkillVersion::new(2, 0, 0));
        assert!(dep.is_satisfied_by(&SkillVersion::new(1, 5, 0)));
        assert!(!dep.is_satisfied_by(&SkillVersion::new(2, 1, 0)));
    }

    #[test]
    fn test_dep_optional() {
        let dep = SkillDep::optional("opt");
        assert!(dep.optional);
    }

    #[test]
    fn test_registry_register_and_query() {
        let mut reg = RegistryV2::new();
        reg.register("skill-a", SkillVersion::new(1, 0, 0), vec![]);
        reg.register("skill-b", SkillVersion::new(2, 3, 1), vec![]);
        assert_eq!(reg.skill_count(), 2);
        assert_eq!(reg.version_of("skill-a"), Some(SkillVersion::new(1, 0, 0)));
        assert_eq!(reg.version_of("unknown"), None);
    }

    #[test]
    fn test_registry_deprecate() {
        let mut reg = RegistryV2::new();
        reg.register("old", SkillVersion::new(0, 1, 0), vec![]);
        assert!(reg.deprecate("old"));
        assert!(reg.is_deprecated("old"));
        assert!(!reg.deprecate("nonexistent"));
    }

    #[test]
    fn test_registry_check_deps_ok() {
        let mut reg = RegistryV2::new();
        reg.register("base", SkillVersion::new(1, 0, 0), vec![]);
        reg.register(
            "consumer",
            SkillVersion::new(1, 0, 0),
            vec![SkillDep::required("base")],
        );
        assert!(reg.check_dependencies("consumer").is_ok());
    }

    #[test]
    fn test_registry_check_deps_missing() {
        let mut reg = RegistryV2::new();
        reg.register(
            "consumer",
            SkillVersion::new(1, 0, 0),
            vec![SkillDep::required("missing")],
        );
        assert!(reg.check_dependencies("consumer").is_err());
    }

    #[test]
    fn test_registry_check_deps_version_mismatch() {
        let mut reg = RegistryV2::new();
        reg.register("base", SkillVersion::new(1, 0, 0), vec![]);
        reg.register(
            "consumer",
            SkillVersion::new(1, 0, 0),
            vec![SkillDep::required("base").with_min_version(SkillVersion::new(2, 0, 0))],
        );
        assert!(reg.check_dependencies("consumer").is_err());
    }

    #[test]
    fn test_conflict_detection() {
        let mut reg = RegistryV2::new();
        reg.register("a", SkillVersion::new(1, 0, 0), vec![]);
        reg.register("b", SkillVersion::new(1, 0, 0), vec![]);
        reg.declare_conflict(SkillConflict::new("a", "b", "incompatible APIs"));
        assert!(!reg.can_coexist("a", "b"));
        assert_eq!(reg.conflicts_for("a").len(), 1);
    }

    #[test]
    fn test_no_conflict() {
        let reg = RegistryV2::new();
        assert!(reg.can_coexist("x", "y"));
    }

    #[test]
    fn test_compat_matrix() {
        let mut cm = SkillCompat::new();
        cm.set("a", "b", true);
        cm.set("a", "c", false);
        assert!(cm.is_compatible("a", "b"));
        assert!(!cm.is_compatible("a", "c"));
        assert!(cm.is_compatible("b", "c")); // no entry, assumed compatible
    }

    #[test]
    fn test_compat_incompatible_with() {
        let mut cm = SkillCompat::new();
        cm.set("a", "b", false);
        cm.set("a", "c", false);
        let incompat: HashSet<&str> = cm.incompatible_with("a").into_iter().collect();
        assert_eq!(incompat.len(), 2);
        assert!(incompat.contains("b"));
        assert!(incompat.contains("c"));
    }

    #[test]
    fn test_sync_common_skills() {
        let mut sync = RegistrySync::new();
        let mut r1 = RegistryV2::new();
        r1.register("shared", SkillVersion::new(1, 0, 0), vec![]);
        r1.register("only-r1", SkillVersion::new(1, 0, 0), vec![]);
        let mut r2 = RegistryV2::new();
        r2.register("shared", SkillVersion::new(1, 0, 0), vec![]);
        r2.register("only-r2", SkillVersion::new(1, 0, 0), vec![]);
        sync.add_room("r1", r1);
        sync.add_room("r2", r2);
        let common = sync.common_skills();
        assert!(common.contains("shared"));
        assert!(!common.contains("only-r1"));
    }

    #[test]
    fn test_sync_version_mismatches() {
        let mut sync = RegistrySync::new();
        let mut r1 = RegistryV2::new();
        r1.register("skill", SkillVersion::new(1, 0, 0), vec![]);
        let mut r2 = RegistryV2::new();
        r2.register("skill", SkillVersion::new(2, 0, 0), vec![]);
        sync.add_room("r1", r1);
        sync.add_room("r2", r2);
        let mismatches = sync.version_mismatches();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].0, "skill");
    }

    #[test]
    fn test_sync_unique_skills() {
        let mut sync = RegistrySync::new();
        let mut r1 = RegistryV2::new();
        r1.register("unique-to-r1", SkillVersion::new(1, 0, 0), vec![]);
        let mut r2 = RegistryV2::new();
        r2.register("unique-to-r2", SkillVersion::new(1, 0, 0), vec![]);
        sync.add_room("r1", r1);
        sync.add_room("r2", r2);
        let unique = sync.unique_skills();
        assert!(unique.contains_key("r1"));
        assert!(unique.contains_key("r2"));
    }

    #[test]
    fn test_sync_no_rooms() {
        let sync = RegistrySync::new();
        assert!(sync.common_skills().is_empty());
        assert_eq!(sync.room_count(), 0);
    }

    #[test]
    fn test_conflict_involves() {
        let c = SkillConflict::new("a", "b", "reason");
        assert!(c.involves("a"));
        assert!(c.involves("b"));
        assert!(!c.involves("c"));
    }
}
