//! HATEOAS Link Types
//!
//! Types for hypermedia links in API responses. Responses can include
//! `_links` to let clients discover available actions dynamically.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A hypermedia link describing an available action or related resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Link {
    /// The URL for this link (absolute or relative path).
    pub href: String,

    /// HTTP method to use. Defaults to GET if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Human-readable title for this action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl Link {
    /// Create a GET link.
    pub fn get(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            method: None,
            title: None,
        }
    }

    /// Create a POST link.
    pub fn post(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            method: Some("POST".into()),
            title: None,
        }
    }

    /// Create a DELETE link.
    pub fn delete(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            method: Some("DELETE".into()),
            title: None,
        }
    }

    /// Create a PATCH link.
    pub fn patch(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            method: Some("PATCH".into()),
            title: None,
        }
    }

    /// Add a title to this link.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// A collection of named links.
pub type Links = HashMap<String, Link>;

/// Builder for constructing link collections.
#[derive(Debug, Default)]
pub struct LinksBuilder {
    links: Links,
}

impl LinksBuilder {
    /// Create a new links builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a link with the given relation name.
    pub fn add(mut self, rel: impl Into<String>, link: Link) -> Self {
        self.links.insert(rel.into(), link);
        self
    }

    /// Add a "self" link.
    pub fn self_link(self, href: impl Into<String>) -> Self {
        self.add("self", Link::get(href))
    }

    /// Build the links map.
    pub fn build(self) -> Links {
        self.links
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_builders() {
        let link = Link::get("/foo").with_title("Get Foo");
        assert_eq!(link.href, "/foo");
        assert_eq!(link.method, None);
        assert_eq!(link.title, Some("Get Foo".into()));

        let link = Link::post("/bar");
        assert_eq!(link.method, Some("POST".into()));
    }

    #[test]
    fn test_links_builder() {
        let links = LinksBuilder::new()
            .self_link("/api/v1/things/123")
            .add("children", Link::get("/api/v1/things/123/children"))
            .add(
                "delete",
                Link::delete("/api/v1/things/123").with_title("Delete"),
            )
            .build();

        assert_eq!(links.len(), 3);
        assert!(links.contains_key("self"));
        assert!(links.contains_key("children"));
        assert!(links.contains_key("delete"));
    }
}
