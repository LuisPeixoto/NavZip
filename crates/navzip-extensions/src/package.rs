use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Component, Path},
};

pub const MAX_PACKAGE_BYTES: u64 = 16 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub origins: Vec<String>,
    pub module: Option<String>,
    pub theme: Option<Theme>,
    #[serde(default)]
    pub scripts: Vec<ContentScript>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub accent: [u8; 3],
    pub dark: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentScript {
    pub matches: Vec<String>,
    pub js: Option<String>,
    pub css: Option<String>,
}
impl Manifest {
    pub fn validate(&self) -> Result<()> {
        if self.manifest_version != 1
            || self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            bail!("Identificação ou versão do manifesto inválida");
        }
        if self.name.trim().is_empty()
            || self.name.len() > 100
            || self.version.is_empty()
            || self.version.len() > 32
        {
            bail!("Nome ou versão inválidos");
        }
        for p in &self.permissions {
            if ![
                "tabs.read",
                "tabs.create",
                "tabs.close",
                "commands",
                "theme",
                "storage",
                "content.scripts",
            ]
            .contains(&p.as_str())
            {
                bail!("Permissão não suportada: {p}");
            }
        }
        for origin in &self.origins {
            validate_pattern(origin)?;
        }
        if let Some(module) = &self.module {
            validate_path(module)?;
        }
        if self.theme.is_some() && !self.permissions.iter().any(|p| p == "theme") {
            bail!("Tema sem permissão");
        }
        for script in &self.scripts {
            if !self.permissions.iter().any(|p| p == "content.scripts") {
                bail!("Script sem permissão");
            }
            for pattern in &script.matches {
                validate_pattern(pattern)?;
                if !self.origins.contains(pattern) {
                    bail!("O script solicita uma origem não declarada");
                }
            }
            for resource in [&script.js, &script.css].into_iter().flatten() {
                validate_path(resource)?;
            }
        }
        Ok(())
    }
    pub fn permits(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
}
pub fn validate_path(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 240
        || s.contains('\\')
        || s.contains(':')
        || Path::new(s)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        bail!("Caminho inseguro no pacote");
    }
    for component in s.split('/') {
        let upper = component
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if component.starts_with('.')
            || component.ends_with(['.', ' '])
            || [
                "CON", "AUX", "NUL", "PRN", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ]
            .contains(&upper.as_str())
        {
            bail!("Nome reservado no pacote");
        }
    }
    Ok(())
}
fn validate_pattern(pattern: &str) -> Result<()> {
    let normalized = pattern.replace("*.", "");
    let u = url::Url::parse(&normalized)?;
    if !matches!(u.scheme(), "https" | "http")
        || u.host_str().is_none()
        || u.password().is_some()
        || !u.username().is_empty()
        || u.query().is_some()
        || u.fragment().is_some()
        || !u.path().ends_with('*')
    {
        bail!("Padrão de origem inválido");
    }
    if pattern.matches('*').count() > if pattern.contains("://*.") { 2 } else { 1 } {
        bail!("Curinga não suportado");
    }
    Ok(())
}
pub fn matches_origin(pattern: &str, input: &str) -> bool {
    if validate_pattern(pattern).is_err() {
        return false;
    }
    let Ok(p) = url::Url::parse(&pattern.replace("*.", "")) else {
        return false;
    };
    let Ok(u) = url::Url::parse(input) else {
        return false;
    };
    if p.scheme() != u.scheme() || p.port_or_known_default() != u.port_or_known_default() {
        return false;
    }
    let (Some(a), Some(b)) = (p.host_str(), u.host_str()) else {
        return false;
    };
    let host = a == b || (pattern.contains("://*.") && b.ends_with(&format!(".{a}")));
    host && u.path().starts_with(p.path().trim_end_matches('*'))
}
/// Inspect all members before extraction. ZIP symlinks, aliases and traversal are rejected.
pub fn inspect(path: &Path) -> Result<Manifest> {
    let mut zip = read_archive(path)?;
    inspect_archive(&mut zip)
}
fn read_archive(path: &Path) -> Result<zip::ZipArchive<std::io::Cursor<Vec<u8>>>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_PACKAGE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        bail!("Pacote maior que 16 MiB");
    }
    Ok(zip::ZipArchive::new(std::io::Cursor::new(bytes))?)
}
fn inspect_archive(zip: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>) -> Result<Manifest> {
    if zip.len() > 128 {
        bail!("Muitos arquivos no pacote");
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name()?.into_owned();
        validate_path(&name)?;
        if !names.insert(name.to_ascii_lowercase()) {
            bail!("Nome de arquivo duplicado");
        }
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            bail!("Links simbólicos não são permitidos");
        }
        total = total.checked_add(file.size()).context("Tamanho inválido")?;
        if total > MAX_PACKAGE_BYTES {
            bail!("Pacote expandido maior que 16 MiB");
        }
    }
    let mut json = String::new();
    zip.by_name("manifest.json")?
        .take(65537)
        .read_to_string(&mut json)?;
    if json.len() > 65536 {
        bail!("Manifesto muito grande");
    }
    let manifest: Manifest = serde_json::from_str(&json)?;
    manifest.validate()?;
    for path in manifest.module.iter().chain(
        manifest
            .scripts
            .iter()
            .flat_map(|s| s.js.iter().chain(s.css.iter())),
    ) {
        if !names.contains(&path.to_ascii_lowercase()) {
            bail!("Recurso ausente: {path}");
        }
    }
    Ok(manifest)
}
pub fn install(path: &Path, root: &Path, consent: &Manifest) -> Result<Manifest> {
    // Inspection and extraction operate on the same immutable bounded byte buffer.
    let mut zip = read_archive(path)?;
    let manifest = inspect_archive(&mut zip)?;
    if &manifest != consent {
        bail!("O pacote mudou desde a solicitação de permissões. Selecione-o novamente.");
    }
    fs::create_dir_all(root)?;
    let stage = tempfile::Builder::new()
        .prefix(".install-")
        .tempdir_in(root)?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name()?.into_owned();
        validate_path(&name)?;
        let destination = stage.path().join(name);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(destination)?;
        let n = std::io::copy(&mut file.by_ref().take(MAX_PACKAGE_BYTES + 1), &mut out)?;
        total = total.checked_add(n).context("Tamanho inválido")?;
        if total > MAX_PACKAGE_BYTES {
            bail!("Limite de extração excedido");
        }
        out.flush()?;
    }
    // A new installation is disabled until the host records the user's permission grant.
    fs::write(stage.path().join(".enabled"), b"false")?;
    let target = root.join(&manifest.id);
    if target.exists() {
        bail!("Remova a extensão existente antes de instalar outra versão");
    }
    fs::rename(stage.path(), target)?;
    Ok(manifest)
}
pub fn installed(root: &Path) -> Vec<(Manifest, bool)> {
    let Ok(entries) = fs::read_dir(root) else {
        return vec![];
    };
    entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_symlink() || !path.is_dir() {
                return None;
            }
            let m: Manifest =
                serde_json::from_slice(&fs::read(path.join("manifest.json")).ok()?).ok()?;
            m.validate().ok()?;
            if path.file_name()?.to_str()? != m.id {
                return None;
            }
            let enabled = fs::read_to_string(path.join(".enabled")).unwrap_or_default() == "true";
            Some((m, enabled))
        })
        .collect()
}
pub fn set_enabled(root: &Path, manifest: &Manifest, enabled: bool) -> Result<()> {
    manifest.validate()?;
    fs::write(
        root.join(&manifest.id).join(".enabled"),
        if enabled { "true" } else { "false" },
    )?;
    Ok(())
}
pub fn remove(root: &Path, manifest: &Manifest) -> Result<()> {
    manifest.validate()?;
    fs::remove_dir_all(root.join(&manifest.id))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn package(path: &Path, manifest: &Manifest) -> Result<()> {
        let mut zip = zip::ZipWriter::new(fs::File::create(path)?);
        zip.start_file("manifest.json", zip::write::SimpleFileOptions::default())?;
        zip.write_all(&serde_json::to_vec(manifest)?)?;
        zip.finish()?;
        Ok(())
    }
    #[test]
    fn consent_cannot_be_reused_for_changed_manifest() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("theme.navzip-extension");
        let consent: Manifest = serde_json::from_str(include_str!(
            "../../../examples/extensions/ocean-theme/manifest.json"
        ))?;
        package(&path, &consent)?;
        let root = dir.path().join("extensions");
        let mut changed = consent.clone();
        changed.permissions.push("tabs.close".into());
        package(&path, &changed)?;
        assert!(install(&path, &root, &consent).is_err());
        assert!(!root.exists());
        package(&path, &consent)?;
        install(&path, &root, &consent)?;
        assert!(!installed(&root)[0].1);
        set_enabled(&root, &consent, true)?;
        assert!(installed(&root)[0].1);
        remove(&root, &consent)?;
        assert!(installed(&root).is_empty());
        Ok(())
    }
    #[test]
    fn malformed_archive_is_rejected() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("bad.navzip-extension");
        fs::write(&path, b"not a zip")?;
        assert!(inspect(&path).is_err());
        Ok(())
    }
    #[test]
    fn origin_matching_does_not_accept_lookalike_hosts() {
        assert!(matches_origin(
            "https://*.example.org/*",
            "https://a.example.org/test"
        ));
        assert!(!matches_origin(
            "https://*.example.org/*",
            "https://example.org.evil.test/"
        ));
        assert!(!matches_origin(
            "https://example.org/*",
            "https://example.org:8443/"
        ));
        assert!(!matches_origin(
            "https://example.org/*",
            "http://example.org/"
        ));
    }
    #[test]
    fn traversal_paths_are_rejected() {
        for p in [
            "../secret",
            "/etc/passwd",
            "a/../../b",
            "C:/file",
            "a\\file",
        ] {
            assert!(validate_path(p).is_err(), "{p}");
        }
    }
}
