//! Source-only preview artifacts. Never substitutes source archives for installers.
use anyhow::{Context, Result, bail};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let result = Command::new("git").current_dir(root).args(args).output()?;
    if !result.status.success() {
        bail!(
            "Git release check failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Ok(String::from_utf8(result.stdout)?.trim().to_owned())
}

fn validate_tag(tag: &str) -> Result<()> {
    if tag != format!("v{}", env!("CARGO_PKG_VERSION")) {
        bail!(
            "Source preview tag must match Cargo version: v{}",
            env!("CARGO_PKG_VERSION")
        );
    }
    Ok(())
}

pub fn prepare(root: &Path, tag: &str) -> Result<()> {
    validate_tag(tag)?;
    if !git(root, &["status", "--porcelain", "--untracked-files=normal"])?.is_empty() {
        bail!("Commit or preserve all source changes before preparing a release");
    }
    let commit = git(root, &["rev-parse", "HEAD"])?;
    if let Ok(existing) = git(
        root,
        &[
            "rev-parse",
            "--verify",
            &format!("refs/tags/{tag}^{{commit}}"),
        ],
    ) && existing != commit
    {
        bail!("Existing release tag does not point to the checked-out commit");
    }
    let engine: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("chromium/engine.lock.json"))?)?;
    let output = root.join("dist/releases").join(tag);
    fs::create_dir_all(output.parent().context("Missing release parent")?)?;
    fs::create_dir(&output)
        .context("Release output already exists; preserve it and use a clean build environment")?;
    let prefix = format!("NavZip-{}/", env!("CARGO_PKG_VERSION"));
    let mut artifacts = vec![];
    for format in ["tar.gz", "zip"] {
        let name = format!("NavZip-{}-source.{format}", env!("CARGO_PKG_VERSION"));
        let destination = output.join(&name);
        let status = Command::new("git")
            .current_dir(root)
            .args([
                "archive",
                &format!("--format={format}"),
                &format!("--prefix={prefix}"),
                "--output",
            ])
            .arg(&destination)
            .arg(&commit)
            .status()?;
        if !status.success() {
            bail!("Source archive generation failed; no release should be published");
        }
        let bytes = fs::read(&destination)?;
        artifacts.push(json!({"file": name, "sha256": format!("{:x}", Sha256::digest(&bytes)), "bytes": bytes.len(), "kind": "source"}));
    }
    let manifest = json!({
        "schemaVersion": 1, "version": env!("CARGO_PKG_VERSION"), "tag": tag,
        "commit": commit, "stage": "experimental-source-preview", "engine": engine,
        "nativeInstallersIncluded": false, "artifacts": artifacts,
        "platforms": {"macos": "pending standalone build and qualification", "windows": "pending native build and qualification", "linux": "pending native build and qualification"}
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    fs::write(output.join("release-manifest.json"), &manifest_bytes)?;
    let mut checksums = artifacts
        .iter()
        .map(|entry| {
            format!(
                "{}  {}\n",
                entry["sha256"].as_str().unwrap(),
                entry["file"].as_str().unwrap()
            )
        })
        .collect::<String>();
    checksums.push_str(&format!(
        "{:x}  release-manifest.json\n",
        Sha256::digest(&manifest_bytes)
    ));
    fs::write(output.join("SHA256SUMS"), checksums)?;
    println!(
        "Source-only preview prepared at {}. No macOS, Windows or Linux installer is included.",
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Result<tempfile::TempDir> {
        let folder = tempfile::tempdir()?;
        git(folder.path(), &["init", "-b", "main"])?;
        fs::create_dir(folder.path().join("chromium"))?;
        fs::write(
            folder.path().join("chromium/engine.lock.json"),
            "{\"product\":\"NavZip\"}",
        )?;
        fs::write(folder.path().join(".gitignore"), "dist/\n")?;
        git(folder.path(), &["add", "."])?;
        git(
            folder.path(),
            &[
                "-c",
                "user.name=Release fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "fixture",
            ],
        )?;
        Ok(folder)
    }

    #[test]
    fn refuses_wrong_version_or_path_in_a_tag() {
        assert!(validate_tag("v1.0.0").is_err());
        assert!(validate_tag("../../outside").is_err());
        assert!(validate_tag(&format!("v{}", env!("CARGO_PKG_VERSION"))).is_ok());
    }

    #[test]
    fn emits_source_archives_with_verifiable_hashes_and_no_installer_claim() -> Result<()> {
        let folder = fixture()?;
        let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
        prepare(folder.path(), &tag)?;
        let output = folder.path().join("dist/releases").join(&tag);
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("release-manifest.json"))?)?;
        assert_eq!(manifest["nativeInstallersIncluded"], false);
        let entries = manifest["artifacts"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        for entry in entries {
            let bytes = fs::read(output.join(entry["file"].as_str().unwrap()))?;
            assert!(!bytes.is_empty());
            assert_eq!(entry["sha256"], format!("{:x}", Sha256::digest(&bytes)));
            assert_eq!(entry["kind"], "source");
        }
        assert!(prepare(folder.path(), &tag).is_err());
        Ok(())
    }

    #[test]
    fn refuses_uncommitted_source_and_conflicting_tag() -> Result<()> {
        let folder = fixture()?;
        let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
        git(folder.path(), &["tag", &tag])?;
        fs::write(folder.path().join("README.md"), "Changed source")?;
        assert!(prepare(folder.path(), &tag).is_err());
        git(folder.path(), &["add", "."])?;
        git(
            folder.path(),
            &[
                "-c",
                "user.name=Release fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "changed",
            ],
        )?;
        assert!(prepare(folder.path(), &tag).is_err());
        assert!(!folder.path().join("dist/releases").exists());
        Ok(())
    }
}
