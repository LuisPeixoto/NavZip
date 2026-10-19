mod alpha;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, backup::Backup};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Export {
    schema_version: u32,
    source_id: String,
    bookmarks: Vec<Bookmark>,
    history: Vec<Visit>,
    settings: alpha::Settings,
    tabs: Vec<alpha::Tab>,
    #[serde(default)]
    active_tab_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bookmark {
    id: String,
    url: String,
    title: String,
    folder: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Visit {
    url: String,
    title: String,
    visited_at: i64,
}

pub fn validate(export: &Export) -> Result<()> {
    if export.schema_version != 1
        || export.source_id.len() != 64
        || !export.source_id.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("Versão ou identidade de migração inválida");
    }
    if export.bookmarks.len() > 100_000
        || export.history.len() > 500_000
        || export.tabs.len() > 1_000
    {
        bail!("Perfil excede os limites de importação");
    }
    for url in export
        .bookmarks
        .iter()
        .map(|b| &b.url)
        .chain(export.history.iter().map(|v| &v.url))
    {
        alpha::page_url(url)?;
    }
    for tab in &export.tabs {
        if tab.url != alpha::HOME {
            alpha::page_url(&tab.url)?;
        }
    }
    if let Some(active) = &export.active_tab_id
        && !export.tabs.iter().any(|tab| tab.id.to_string() == *active)
    {
        bail!("Aba ativa não pertence à sessão exportada");
    }
    Ok(())
}

pub fn export(source: &Path, destination: &Path) -> Result<PathBuf> {
    // Never open the live database with write flags or copy a live WAL pair.
    let source = source
        .canonicalize()
        .context("Perfil legado não encontrado")?;
    let db =
        Connection::open_with_flags(source.join("library.db"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    if db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))? != 1 {
        bail!("Schema legado não suportado");
    }
    fs::create_dir_all(destination.parent().unwrap_or(Path::new(".")))?;
    fs::create_dir(destination)
        .context("Use um diretório de saída novo; backups existentes não são sobrescritos")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(destination, fs::Permissions::from_mode(0o700))?;
    }
    let mut snapshot = Connection::open(destination.join("library.backup.db"))?;
    Backup::new(&db, &mut snapshot)?.run_to_completion(128, Duration::from_millis(10), None)?;
    // A stable profile identity, independent of the current file contents.
    let source_id = format!("{:x}", Sha256::digest(source.to_string_lossy().as_bytes()));
    let bookmarks = snapshot
        .prepare("SELECT id,url,title,folder FROM bookmarks ORDER BY id")?
        .query_map([], |r| {
            Ok(Bookmark {
                id: r.get(0)?,
                url: r.get(1)?,
                title: r.get(2)?,
                folder: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let history = snapshot
        .prepare("SELECT url,title,visited_at FROM history ORDER BY id LIMIT 500001")?
        .query_map([], |r| {
            Ok(Visit {
                url: r.get(0)?,
                title: r.get(1)?,
                visited_at: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    use rusqlite::OptionalExtension;
    let state = |key: &str| -> Result<Option<String>> {
        Ok(snapshot
            .query_row("SELECT value FROM state WHERE key=?1", [key], |r| r.get(0))
            .optional()?)
    };
    let settings = state("settings")?
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    let session = state("session")?
        .map(|s| serde_json::from_str::<alpha::Session>(&s))
        .transpose()?
        .map(|s| s.sanitize());
    let active_tab_id = session.as_ref().map(|s| s.active.to_string());
    let tabs = session.map(|s| s.tabs).unwrap_or_default();
    let export = Export {
        schema_version: 1,
        source_id,
        bookmarks,
        history,
        settings,
        tabs,
        active_tab_id,
    };
    validate(&export)?;
    let output = destination.join("profile.json");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(&serde_json::to_vec_pretty(&export)?)?;
    file.sync_all()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Reproduce the stored Alpha v1 schema directly. Do not depend on its retired
    // writer: this fixture verifies compatibility with previously written data.
    fn source_db(source: &Path) -> Result<Connection> {
        fs::create_dir(source)?;
        let db = Connection::open(source.join("library.db"))?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE bookmarks(id TEXT PRIMARY KEY,url TEXT NOT NULL,title TEXT NOT NULL,folder TEXT NOT NULL);
             CREATE TABLE history(id INTEGER PRIMARY KEY,url TEXT NOT NULL,title TEXT NOT NULL,visited_at INTEGER NOT NULL);
             CREATE TABLE state(key TEXT PRIMARY KEY,value TEXT NOT NULL);
             CREATE TABLE permissions(origin TEXT NOT NULL,feature TEXT NOT NULL,allowed INTEGER NOT NULL,PRIMARY KEY(origin,feature));
             INSERT INTO bookmarks VALUES('26a54c21-fc10-4c60-b287-ae194fe121da','https://example.org/','Exemplo','Trabalho');
             INSERT INTO history VALUES(1,'https://example.org/','Exemplo',123);
             PRAGMA user_version=1;",
        )?;
        Ok(db)
    }

    fn read_export(source: &Path, destination: &Path) -> Result<Export> {
        Ok(serde_json::from_slice(&fs::read(export(
            source,
            destination,
        )?)?)?)
    }

    fn session_fixture() -> serde_json::Value {
        json!({
            "active": "ea4aaacb-acce-49c0-97c5-a70f1d65bc80",
            "tabs": [
                {"id": "19f8b86e-f09b-44d4-839e-f6ce8344c28a", "url": "navzip://newtab",
                 "title": "Nova aba", "pinned": false, "muted": false, "zoom": 1.0},
                {"id": "ea4aaacb-acce-49c0-97c5-a70f1d65bc80", "url": "https://example.org/",
                 "title": "Exemplo", "pinned": true, "muted": true, "zoom": 1.25}
            ]
        })
    }

    #[test]
    fn exports_a_consistent_snapshot_without_changing_source() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("old");
        let db = source_db(&source)?;
        // Keep the connection open: all inserted rows are still in an active WAL.
        let before = fs::read(source.join("library.db"))?;
        let wal = fs::read(source.join("library.db-wal"))?;
        let a = read_export(&source, &temp.path().join("backup1"))?;
        assert_eq!(a.history.len(), 1);
        assert_eq!(a.history[0].visited_at, 123);
        assert_eq!(a.bookmarks[0].folder, "Trabalho");
        let b = read_export(&source, &temp.path().join("backup2"))?;
        assert_eq!(a.source_id, b.source_id);
        assert!(export(&source, &temp.path().join("backup1")).is_err());
        assert_eq!(fs::read(source.join("library.db"))?, before);
        assert_eq!(fs::read(source.join("library.db-wal"))?, wal);
        let backup = Connection::open(temp.path().join("backup1/library.backup.db"))?;
        assert_eq!(
            backup.query_row("SELECT count(*) FROM history", [], |r| r.get::<_, i64>(0))?,
            1
        );
        drop(db);
        Ok(())
    }

    #[test]
    fn refuses_unknown_schema_and_privileged_urls() -> Result<()> {
        let mut data = Export {
            schema_version: 2,
            source_id: "a".repeat(64),
            bookmarks: vec![],
            history: vec![],
            settings: Default::default(),
            tabs: vec![],
            active_tab_id: None,
        };
        assert!(validate(&data).is_err());
        data.schema_version = 1;
        for url in [
            "javascript:alert(1)",
            "navzip://newtab",
            "file:///etc/passwd",
            "https://user:secret@example.org/",
        ] {
            data.history = vec![Visit {
                url: url.into(),
                title: String::new(),
                visited_at: 0,
            }];
            assert!(validate(&data).is_err(), "{url}");
        }
        Ok(())
    }

    #[test]
    fn preserves_settings_tabs_and_the_export_wire_format() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("old");
        let db = source_db(&source)?;
        let session = session_fixture();
        let settings = json!({"theme": "Dark", "tab_layout": "Vertical", "block_trackers": false,
            "restore_session": false, "discard_inactive": true, "search_url": "https://example.org/?q="});
        for (key, value) in [("session", &session), ("settings", &settings)] {
            db.execute("INSERT INTO state VALUES(?1,?2)", [key, &value.to_string()])?;
        }
        let mut data = read_export(&source, &temp.path().join("backup"))?;
        let output = serde_json::to_value(&data)?;
        assert_eq!(output["schemaVersion"], 1);
        assert_eq!(output["settings"], settings);
        assert_eq!(output["tabs"], session["tabs"]);
        assert_eq!(output["activeTabId"], session["active"]);
        assert_eq!(output["history"][0]["visitedAt"], 123);
        data.active_tab_id = Some("does-not-exist".into());
        assert!(validate(&data).is_err());
        Ok(())
    }

    #[test]
    fn normalizes_damaged_sessions_without_restoring_privileged_urls() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("old");
        let db = source_db(&source)?;
        let mut session = session_fixture();
        session["active"] = json!("00000000-0000-0000-0000-000000000000");
        session["tabs"][0]["zoom"] = json!(100);
        let mut duplicate = session["tabs"][0].clone();
        duplicate["zoom"] = json!(0.1);
        let mut invalid = duplicate.clone();
        invalid["url"] = json!("javascript:alert(1)");
        session["tabs"]
            .as_array_mut()
            .unwrap()
            .extend([duplicate, invalid]);
        db.execute(
            "INSERT INTO state VALUES('session',?1)",
            [session.to_string()],
        )?;
        let data = read_export(&source, &temp.path().join("backup"))?;
        assert_eq!(data.tabs.len(), 3);
        assert_ne!(data.tabs[0].id, data.tabs[2].id);
        assert_eq!(data.active_tab_id, Some(data.tabs[0].id.to_string()));
        let output = serde_json::to_value(data)?;
        assert_eq!(output["tabs"][0]["zoom"], 5.0);
        assert_eq!(output["tabs"][2]["zoom"], 0.25);
        Ok(())
    }

    #[test]
    fn supports_missing_state_and_empty_sessions() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("old");
        let db = source_db(&source)?;
        let data = read_export(&source, &temp.path().join("missing"))?;
        assert!(data.tabs.is_empty());
        assert!(data.active_tab_id.is_none());
        assert_eq!(serde_json::to_value(data)?["settings"]["theme"], "System");
        db.execute(
            "INSERT INTO state VALUES('session',?1)",
            [json!({"tabs": [], "active": "00000000-0000-0000-0000-000000000000"}).to_string()],
        )?;
        let data = read_export(&source, &temp.path().join("empty"))?;
        assert_eq!(data.tabs.len(), 1);
        assert_eq!(data.tabs[0].url, alpha::HOME);
        assert_eq!(data.active_tab_id, Some(data.tabs[0].id.to_string()));
        Ok(())
    }

    #[test]
    fn refuses_an_unknown_database_version_without_writing_to_it() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("old");
        let db = source_db(&source)?;
        db.execute_batch("PRAGMA user_version=99; PRAGMA wal_checkpoint(TRUNCATE);")?;
        let before = fs::read(source.join("library.db"))?;
        let destination = temp.path().join("backup");
        assert!(export(&source, &destination).is_err());
        assert!(!destination.exists());
        assert_eq!(fs::read(source.join("library.db"))?, before);
        Ok(())
    }
}
