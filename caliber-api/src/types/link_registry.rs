//! Link Registry
//!
//! Central registry for HATEOAS link patterns. Instead of scattering link
//! generation across response types, define all link patterns here.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::{Link, Links};

/// HTTP methods for link actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Patch,
    Delete,
}

/// A link pattern definition.
#[derive(Debug, Clone)]
pub struct LinkDef {
    /// Relation name (e.g., "self", "scopes", "close")
    pub rel: &'static str,
    /// Path template with {id} placeholder
    pub path: &'static str,
    /// HTTP method
    pub method: Method,
    /// Human-readable title
    pub title: Option<&'static str>,
    /// Optional condition key (checked against entity state)
    pub when: Option<&'static str>,
}

impl LinkDef {
    pub const fn get(rel: &'static str, path: &'static str) -> Self {
        Self {
            rel,
            path,
            method: Method::Get,
            title: None,
            when: None,
        }
    }

    pub const fn post(rel: &'static str, path: &'static str) -> Self {
        Self {
            rel,
            path,
            method: Method::Post,
            title: None,
            when: None,
        }
    }

    pub const fn patch(rel: &'static str, path: &'static str) -> Self {
        Self {
            rel,
            path,
            method: Method::Patch,
            title: None,
            when: None,
        }
    }

    pub const fn delete(rel: &'static str, path: &'static str) -> Self {
        Self {
            rel,
            path,
            method: Method::Delete,
            title: None,
            when: None,
        }
    }

    pub const fn titled(mut self, title: &'static str) -> Self {
        self.title = Some(title);
        self
    }

    pub const fn when(mut self, condition: &'static str) -> Self {
        self.when = Some(condition);
        self
    }
}

/// Entity link configuration.
pub struct EntityDef {
    /// Base path (e.g., "/api/v1/trajectories")
    pub base: &'static str,
    /// Link definitions for this entity
    pub links: &'static [LinkDef],
}

/// Trait for types that can be linked via the registry.
pub trait Linkable {
    /// Entity type name (must match registry key)
    const ENTITY_TYPE: &'static str;

    /// Get the entity's ID as a string for link generation.
    /// Named `link_id` to avoid conflict with CacheableEntity::entity_id.
    fn link_id(&self) -> String;

    /// Check if a condition is satisfied for conditional links.
    /// Return true if the condition name is satisfied.
    fn check_condition(&self, condition: &str) -> bool {
        let _ = condition;
        true // default: all conditions pass
    }

    /// Get related entity IDs for relation links (e.g., parent_id, trajectory_id)
    fn relation_id(&self, relation: &str) -> Option<String> {
        let _ = relation;
        None
    }
}

// ============================================================================
// REGISTRY DEFINITIONS
// ============================================================================

/// Trajectory links
const TRAJECTORY_LINKS: &[LinkDef] = &[
    LinkDef::get("self", "{base}/{id}"),
    LinkDef::get("scopes", "{base}/{id}/scopes").titled("List scopes"),
    LinkDef::get("artifacts", "{base}/{id}/artifacts").titled("List artifacts"),
    LinkDef::get("notes", "{base}/{id}/notes").titled("List notes"),
    LinkDef::get("children", "{base}/{id}/children").titled("Child trajectories"),
    LinkDef::patch("update", "{base}/{id}")
        .titled("Update")
        .when("mutable"),
    LinkDef::post("close", "{base}/{id}/close")
        .titled("Close")
        .when("mutable"),
    LinkDef::get("parent", "/api/v1/trajectories/{parent_id}")
        .titled("Parent")
        .when("has_parent"),
];

/// Scope links
const SCOPE_LINKS: &[LinkDef] = &[
    LinkDef::get("self", "{base}/{id}"),
    LinkDef::get("turns", "{base}/{id}/turns").titled("List turns"),
    LinkDef::get("artifacts", "{base}/{id}/artifacts").titled("List artifacts"),
    LinkDef::get("trajectory", "/api/v1/trajectories/{trajectory_id}").titled("Parent trajectory"),
    LinkDef::patch("update", "{base}/{id}")
        .titled("Update")
        .when("active"),
    LinkDef::post("close", "{base}/{id}/close")
        .titled("Close")
        .when("active"),
    LinkDef::post("checkpoint", "{base}/{id}/checkpoint")
        .titled("Checkpoint")
        .when("active"),
    LinkDef::get("parent", "/api/v1/scopes/{parent_id}")
        .titled("Parent scope")
        .when("has_parent"),
];

/// Artifact links
const ARTIFACT_LINKS: &[LinkDef] = &[
    LinkDef::get("self", "{base}/{id}"),
    LinkDef::patch("update", "{base}/{id}").titled("Update"),
    LinkDef::delete("delete", "{base}/{id}").titled("Delete"),
    LinkDef::get("trajectory", "/api/v1/trajectories/{trajectory_id}").titled("Parent trajectory"),
    LinkDef::get("scope", "/api/v1/scopes/{scope_id}").titled("Parent scope"),
    LinkDef::get("superseded_by", "/api/v1/artifacts/{superseded_by}")
        .titled("Superseding")
        .when("has_superseded"),
];

/// Note links
const NOTE_LINKS: &[LinkDef] = &[
    LinkDef::get("self", "{base}/{id}"),
    LinkDef::patch("update", "{base}/{id}").titled("Update"),
    LinkDef::delete("delete", "{base}/{id}").titled("Delete"),
    LinkDef::get("superseded_by", "/api/v1/notes/{superseded_by}")
        .titled("Superseding")
        .when("has_superseded"),
];

/// The global link registry.
pub static LINK_REGISTRY: LazyLock<LinkRegistry> = LazyLock::new(|| {
    let mut registry = LinkRegistry::new();
    registry.register(
        "trajectory",
        EntityDef {
            base: "/api/v1/trajectories",
            links: TRAJECTORY_LINKS,
        },
    );
    registry.register(
        "scope",
        EntityDef {
            base: "/api/v1/scopes",
            links: SCOPE_LINKS,
        },
    );
    registry.register(
        "artifact",
        EntityDef {
            base: "/api/v1/artifacts",
            links: ARTIFACT_LINKS,
        },
    );
    registry.register(
        "note",
        EntityDef {
            base: "/api/v1/notes",
            links: NOTE_LINKS,
        },
    );
    registry
});

// ============================================================================
// REGISTRY IMPLEMENTATION
// ============================================================================

/// Central registry for link patterns.
pub struct LinkRegistry {
    entities: HashMap<&'static str, EntityDef>,
}

impl LinkRegistry {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn register(&mut self, entity_type: &'static str, def: EntityDef) {
        self.entities.insert(entity_type, def);
    }

    /// Generate links for a linkable entity.
    pub fn generate<T: Linkable>(&self, entity: &T) -> Links {
        let Some(def) = self.entities.get(T::ENTITY_TYPE) else {
            return Links::new();
        };

        let id = entity.link_id();
        let mut links = Links::new();

        for link_def in def.links {
            // Check condition if specified
            if let Some(cond) = link_def.when {
                if !entity.check_condition(cond) {
                    continue;
                }
            }

            // Build the path by replacing placeholders
            let path = self.expand_path(link_def.path, def.base, &id, entity);

            // Skip if path has unresolved placeholders (missing relation)
            if path.contains('{') {
                continue;
            }

            let link = match link_def.method {
                Method::Get => Link::get(path),
                Method::Post => Link::post(path),
                Method::Patch => Link::patch(path),
                Method::Delete => Link::delete(path),
            };

            let link = if let Some(title) = link_def.title {
                link.with_title(title)
            } else {
                link
            };

            links.insert(link_def.rel.to_string(), link);
        }

        links
    }

    fn expand_path<T: Linkable>(&self, template: &str, base: &str, id: &str, entity: &T) -> String {
        let mut path = template.replace("{base}", base).replace("{id}", id);

        // Replace relation placeholders like {parent_id}, {trajectory_id}
        let placeholders: Vec<_> = path
            .match_indices('{')
            .filter_map(|(start, _)| {
                let end = path[start..].find('}')?;
                Some(&path[start + 1..start + end])
            })
            .map(|s| s.to_string())
            .collect();

        for placeholder in placeholders {
            if let Some(relation_id) = entity.relation_id(&placeholder) {
                path = path.replace(&format!("{{{}}}", placeholder), &relation_id);
            }
        }

        path
    }
}

impl Default for LinkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait to add registry-based links to any Linkable type.
pub trait AddLinks: Linkable + Sized {
    fn add_links(self) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEntity {
        id: String,
        is_active: bool,
        parent_id: Option<String>,
    }

    impl Linkable for TestEntity {
        const ENTITY_TYPE: &'static str = "test";

        fn link_id(&self) -> String {
            self.id.clone()
        }

        fn check_condition(&self, condition: &str) -> bool {
            match condition {
                "active" => self.is_active,
                "has_parent" => self.parent_id.is_some(),
                _ => true,
            }
        }

        fn relation_id(&self, relation: &str) -> Option<String> {
            match relation {
                "parent_id" => self.parent_id.clone(),
                _ => None,
            }
        }
    }

    const TEST_LINKS: &[LinkDef] = &[
        LinkDef::get("self", "{base}/{id}"),
        LinkDef::patch("update", "{base}/{id}").when("active"),
        LinkDef::get("parent", "{base}/{parent_id}").when("has_parent"),
    ];

    #[test]
    fn test_registry_generation() {
        let mut registry = LinkRegistry::new();
        registry.register(
            "test",
            EntityDef {
                base: "/api/v1/tests",
                links: TEST_LINKS,
            },
        );

        // Active entity with parent
        let entity = TestEntity {
            id: "123".into(),
            is_active: true,
            parent_id: Some("456".into()),
        };
        let links = registry.generate(&entity);

        assert!(links.contains_key("self"));
        assert!(links.contains_key("update"));
        assert!(links.contains_key("parent"));
        assert_eq!(links["self"].href, "/api/v1/tests/123");
        assert_eq!(links["parent"].href, "/api/v1/tests/456");

        // Inactive entity without parent
        let entity = TestEntity {
            id: "789".into(),
            is_active: false,
            parent_id: None,
        };
        let links = registry.generate(&entity);

        assert!(links.contains_key("self"));
        assert!(!links.contains_key("update")); // condition failed
        assert!(!links.contains_key("parent")); // condition failed
    }
}
