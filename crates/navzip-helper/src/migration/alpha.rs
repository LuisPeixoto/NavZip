//! Data-only compatibility with the Alpha v1 profile/export schema.
//! Chromium owns live tabs, settings and storage; these types only read old data.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

pub(super) const HOME: &str = "navzip://newtab";

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub(super) enum TabLayout {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Settings {
    theme: Theme,
    tab_layout: TabLayout,
    block_trackers: bool,
    restore_session: bool,
    discard_inactive: bool,
    search_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            tab_layout: TabLayout::Horizontal,
            block_trackers: true,
            restore_session: true,
            discard_inactive: false,
            search_url: "https://duckduckgo.com/?q=".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Tab {
    pub id: Uuid,
    pub url: String,
    title: String,
    pinned: bool,
    muted: bool,
    zoom: f32,
}

#[derive(Debug, Deserialize)]
pub(super) struct Session {
    pub tabs: Vec<Tab>,
    pub active: Uuid,
}

impl Session {
    /// Keep the Alpha export's URL, identity and zoom normalization contract.
    pub fn sanitize(mut self) -> Self {
        self.tabs
            .retain(|tab| tab.url == HOME || page_url(&tab.url).is_ok());
        let mut seen = std::collections::HashSet::new();
        for tab in &mut self.tabs {
            if !seen.insert(tab.id) {
                tab.id = Uuid::new_v4();
            }
            tab.zoom = if tab.zoom.is_finite() {
                tab.zoom.clamp(0.25, 5.)
            } else {
                1.
            };
        }
        if self.tabs.is_empty() {
            self.tabs.push(Tab {
                id: Uuid::new_v4(),
                url: HOME.into(),
                title: "Nova aba".into(),
                pinned: false,
                muted: false,
                zoom: 1.,
            });
        }
        if !self.tabs.iter().any(|tab| tab.id == self.active) {
            self.active = self.tabs[0].id;
        }
        self
    }
}

pub(super) fn page_url(input: &str) -> Result<Url> {
    let url = Url::parse(input)?;
    if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
        bail!("Endereço não permitido");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("Use o diálogo de autenticação para credenciais");
    }
    Ok(url)
}
