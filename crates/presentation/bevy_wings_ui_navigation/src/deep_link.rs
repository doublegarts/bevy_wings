//! Deep links: URLs such as `myapp://notes/42` or `/notes/42` open a back stack.
//!
//! ```ignore
//! app.insert_resource(
//!     WingsDeepLinks::<Screen>::new()
//!         .route("/", |_| Some(vec![Screen::Home]))
//!         .route("/notes/:id", |p| Some(vec![Screen::Notes, Screen::Note(p.get("id")?.parse().ok()?)])),
//! );
//! // Platform code (or a test) opens a link:
//! messages.write(WingsOpenDeepLink::new("myapp://notes/42"));
//! ```
//!
//! Links given on the command line (`app myapp://notes/42`) open at startup.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::back_stack::{NavKey, WingsBackStack};

/// Path parameters (`:name` segments) and query parameters of a matched link.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WingsLinkParams {
    values: HashMap<String, String>,
}

impl WingsLinkParams {
    /// A parameter.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }
}

type RouteFn<K> = Arc<dyn Fn(&WingsLinkParams) -> Option<Vec<K>> + Send + Sync>;

/// Routes from link paths to back stacks.
#[derive(Resource)]
pub struct WingsDeepLinks<K: NavKey> {
    routes: Vec<(Vec<String>, RouteFn<K>)>,
}

impl<K: NavKey> Default for WingsDeepLinks<K> {
    fn default() -> Self {
        Self { routes: Vec::new() }
    }
}

/// Splits a link into path segments and query parameters. Custom scheme links use the host
/// as the first segment (`myapp://notes/42` is `/notes/42`); web links drop the host
/// (`https://example.com/notes/42` is `/notes/42`).
pub fn split_link(link: &str) -> (Vec<String>, HashMap<String, String>) {
    let link = link.trim();
    let rest = match link.split_once("://") {
        Some((scheme, rest))
            if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") =>
        {
            rest.split_once('/').map_or("", |(_, path)| path)
        }
        Some((_, rest)) => rest,
        None => link,
    };
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let segments = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let params = query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (!k.is_empty()).then(|| (k.to_string(), v.to_string()))
        })
        .collect();
    (segments, params)
}

impl<K: NavKey> WingsDeepLinks<K> {
    /// No routes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a route. `pattern` is a path with literal segments and `:name` parameters, like
    /// `/notes/:id`; `build` returns the back stack to open, or `None` to reject the link.
    pub fn route(
        mut self,
        pattern: &str,
        build: impl Fn(&WingsLinkParams) -> Option<Vec<K>> + Send + Sync + 'static,
    ) -> Self {
        let (segments, _) = split_link(pattern);
        self.routes.push((segments, Arc::new(build)));
        self
    }

    /// Resolves a link to a back stack.
    pub fn resolve(&self, link: &str) -> Option<Vec<K>> {
        let (segments, query) = split_link(link);
        for (pattern, build) in &self.routes {
            if pattern.len() != segments.len() {
                continue;
            }
            let mut params = WingsLinkParams {
                values: query.clone(),
            };
            let matched = pattern
                .iter()
                .zip(&segments)
                .all(|(p, s)| match p.strip_prefix(':') {
                    Some(name) => {
                        params.values.insert(name.to_string(), s.clone());
                        true
                    }
                    None => p == s,
                });
            if matched
                && let Some(stack) = build(&params)
                && !stack.is_empty()
            {
                return Some(stack);
            }
        }
        None
    }
}

/// Asks to open a link.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct WingsOpenDeepLink {
    /// The link.
    pub link: String,
}

impl WingsOpenDeepLink {
    /// A link to open.
    pub fn new(link: impl Into<String>) -> Self {
        Self { link: link.into() }
    }
}

/// Sent when a link matched no route.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct WingsDeepLinkFailed {
    /// The link.
    pub link: String,
}

/// Opens requested links by replacing the back stack.
pub fn open_deep_links<K: NavKey>(
    links: Option<Res<WingsDeepLinks<K>>>,
    mut requests: MessageReader<WingsOpenDeepLink>,
    mut failed: MessageWriter<WingsDeepLinkFailed>,
    mut stack: ResMut<WingsBackStack<K>>,
) {
    let Some(links) = links else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        match links.resolve(&request.link) {
            Some(entries) => *stack = WingsBackStack::from_entries(entries),
            None => {
                failed.write(WingsDeepLinkFailed {
                    link: request.link.clone(),
                });
            }
        }
    }
}

/// Opens a link passed on the command line (`scheme://...` or `/path`) at startup.
pub fn open_command_line_link(mut writer: MessageWriter<WingsOpenDeepLink>) {
    if let Some(link) = std::env::args()
        .skip(1)
        .find(|a| a.contains("://") && !a.starts_with("file://"))
    {
        writer.write(WingsOpenDeepLink::new(link));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum Screen {
        Home,
        Notes,
        Note(u32),
    }

    fn links() -> WingsDeepLinks<Screen> {
        WingsDeepLinks::new()
            .route("/", |_| Some(vec![Screen::Home]))
            .route("/notes", |_| Some(vec![Screen::Home, Screen::Notes]))
            .route("/notes/:id", |p| {
                Some(vec![
                    Screen::Home,
                    Screen::Notes,
                    Screen::Note(p.get("id")?.parse().ok()?),
                ])
            })
    }

    /// Schemes, hosts and query strings are handled; parameters are parsed.
    #[test]
    fn resolve_routes() {
        let l = links();
        assert_eq!(
            l.resolve("myapp://notes/42"),
            Some(vec![Screen::Home, Screen::Notes, Screen::Note(42)])
        );
        assert_eq!(
            l.resolve("/notes?sort=date"),
            Some(vec![Screen::Home, Screen::Notes])
        );
        assert_eq!(l.resolve("myapp://"), Some(vec![Screen::Home]));
        assert_eq!(l.resolve("/notes/abc"), None);
        assert_eq!(l.resolve("/settings"), None);
        assert_eq!(
            l.resolve("https://example.com/notes/7"),
            Some(vec![Screen::Home, Screen::Notes, Screen::Note(7)])
        );
        let (_, q) = split_link("/a?x=1&y");
        assert_eq!(q.get("x").map(String::as_str), Some("1"));
        assert_eq!(q.get("y").map(String::as_str), Some(""));
    }
}
