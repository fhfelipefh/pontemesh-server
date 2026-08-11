use crate::config::PontemeshHome;
use anyhow::{Context, bail};
use flate2::read::GzDecoder;
use reqwest::{Client, redirect::Policy};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub const UPDATE_REPOSITORY: &str = "fhfelipefh/pontemesh-server";
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 200 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseManifest {
    schema: u8,
    product: String,
    version: String,
    assets: Vec<ManifestAsset>,
}

#[derive(Debug, Deserialize)]
struct ManifestAsset {
    name: String,
    size: u64,
    sha256: String,
}

pub async fn latest_release() -> anyhow::Result<GithubRelease> {
    update_client()?
        .get(format!(
            "https://api.github.com/repos/{UPDATE_REPOSITORY}/releases/latest"
        ))
        .header(reqwest::header::USER_AGENT, "pontemesh-server-update-check")
        .send()
        .await?
        .error_for_status()?
        .json::<GithubRelease>()
        .await
        .context("failed to parse the latest server release")
}

pub async fn stage_and_spawn(
    paths: &PontemeshHome,
    release: &GithubRelease,
    version: &str,
) -> anyhow::Result<()> {
    let stage_dir = paths.state_dir().join("updates").join(version);
    tokio::fs::create_dir_all(&stage_dir)
        .await
        .with_context(|| {
            format!(
                "failed to create update staging directory {}",
                stage_dir.display()
            )
        })?;
    let manifest_name = format!("pontemesh-server-v{version}-manifest.json");
    let archive_name = platform_asset_name(version);
    let manifest_release_asset = find_release_asset(release, &manifest_name)?;
    let archive_release_asset = find_release_asset(release, &archive_name)?;
    let client = update_client()?;
    let manifest_bytes = download_limited(
        &client,
        &manifest_release_asset.browser_download_url,
        MAX_MANIFEST_BYTES,
    )
    .await?;
    let manifest: ReleaseManifest =
        serde_json::from_slice(&manifest_bytes).context("release manifest is not valid JSON")?;
    let manifest_asset = validate_manifest(&manifest, version, &archive_name)?;
    let archive_bytes = download_limited(
        &client,
        &archive_release_asset.browser_download_url,
        MAX_ARCHIVE_BYTES,
    )
    .await?;
    validate_archive(&archive_bytes, manifest_asset)?;
    let archive_path = stage_dir.join(&archive_name);
    tokio::fs::write(&archive_path, &archive_bytes)
        .await
        .with_context(|| format!("failed to stage update archive {}", archive_path.display()))?;
    let staged_executable = stage_dir.join(platform_executable_name());
    let archive_for_extract = archive_path.clone();
    let executable_for_extract = staged_executable.clone();
    tokio::task::spawn_blocking(move || {
        extract_executable(&archive_for_extract, &executable_for_extract)
    })
    .await
    .context("update extraction task failed")??;

    let current_executable =
        std::env::current_exe().context("failed to locate the running executable")?;
    let permissions = fs::metadata(&current_executable)?.permissions();
    fs::set_permissions(&staged_executable, permissions)
        .context("failed to apply executable permissions to staged update")?;
    let helper = stage_dir.join(format!(
        "update-helper-{}{}",
        uuid::Uuid::new_v4().simple(),
        std::env::consts::EXE_SUFFIX
    ));
    fs::copy(&current_executable, &helper)
        .with_context(|| format!("failed to create update helper {}", helper.display()))?;
    let mut command = Command::new(&helper);
    command
        .arg("apply-staged-update")
        .arg("--current-executable")
        .arg(&current_executable)
        .arg("--staged-executable")
        .arg(&staged_executable);
    detach(&mut command);
    command
        .spawn()
        .context("failed to start built-in updater")?;
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        std::process::exit(0);
    });
    Ok(())
}

pub fn run_apply_helper(args: &[String]) -> anyhow::Result<()> {
    let current_executable = argument_path(args, "--current-executable")?;
    let staged_executable = argument_path(args, "--staged-executable")?;
    if !current_executable.is_absolute() || !staged_executable.is_absolute() {
        bail!("update executable paths must be absolute");
    }
    if !staged_executable.is_file() {
        bail!("staged update executable does not exist");
    }
    std::thread::sleep(Duration::from_secs(4));
    let backup = current_executable.with_extension(format!(
        "{}previous",
        current_executable
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!("{value}."))
            .unwrap_or_default()
    ));
    if backup.exists() {
        fs::remove_file(&backup).with_context(|| {
            format!(
                "failed to remove previous update backup {}",
                backup.display()
            )
        })?;
    }
    fs::rename(&current_executable, &backup).with_context(|| {
        format!(
            "failed to back up current executable {}",
            current_executable.display()
        )
    })?;
    if let Err(error) = fs::rename(&staged_executable, &current_executable) {
        let _ = fs::rename(&backup, &current_executable);
        return Err(anyhow::Error::new(error).context("failed to install staged executable"));
    }
    if let Err(error) = Command::new(&current_executable).spawn() {
        let _ = fs::remove_file(&current_executable);
        let _ = fs::rename(&backup, &current_executable);
        return Err(anyhow::Error::new(error).context("failed to restart updated server"));
    }
    Ok(())
}

fn update_client() -> anyhow::Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .redirect(Policy::limited(5))
        .build()
        .context("failed to create server update client")
}

async fn download_limited(client: &Client, url: &str, limit: usize) -> anyhow::Result<Vec<u8>> {
    let response = client.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        bail!("update asset exceeds the allowed size");
    }
    let bytes = response.bytes().await?;
    if bytes.len() > limit {
        bail!("update asset exceeds the allowed size");
    }
    Ok(bytes.to_vec())
}

fn find_release_asset<'a>(
    release: &'a GithubRelease,
    name: &str,
) -> anyhow::Result<&'a GithubReleaseAsset> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .with_context(|| format!("release asset not found: {name}"))
}

fn validate_manifest<'a>(
    manifest: &'a ReleaseManifest,
    version: &str,
    asset_name: &str,
) -> anyhow::Result<&'a ManifestAsset> {
    if manifest.schema != 1 || manifest.product != "server" || manifest.version != version {
        bail!("release manifest identity mismatch");
    }
    manifest
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .with_context(|| format!("release manifest does not contain {asset_name}"))
}

fn validate_archive(bytes: &[u8], asset: &ManifestAsset) -> anyhow::Result<()> {
    if bytes.len() as u64 != asset.size {
        bail!("downloaded update size does not match the release manifest");
    }
    let digest = format!("{:x}", Sha256::digest(bytes));
    if !digest.eq_ignore_ascii_case(&asset.sha256) {
        bail!("downloaded update SHA-256 does not match the release manifest");
    }
    Ok(())
}

fn extract_executable(archive_path: &Path, destination: &Path) -> anyhow::Result<()> {
    if archive_path
        .extension()
        .is_some_and(|extension| extension == "zip")
    {
        return extract_zip_executable(archive_path, destination);
    }
    extract_tar_executable(archive_path, destination)
}

fn extract_zip_executable(archive_path: &Path, destination: &Path) -> anyhow::Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_file()
            && Path::new(entry.name())
                .file_name()
                .is_some_and(|name| name == platform_executable_name())
        {
            let mut output = File::create(destination)?;
            io::copy(&mut entry, &mut output)?;
            return Ok(());
        }
    }
    bail!("server executable was not found in the update archive")
}

fn extract_tar_executable(archive_path: &Path, destination: &Path) -> anyhow::Result<()> {
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if entry.header().entry_type().is_file()
            && path
                .file_name()
                .is_some_and(|name| name == platform_executable_name())
        {
            let mut output = File::create(destination)?;
            io::copy(&mut entry, &mut output)?;
            return Ok(());
        }
    }
    bail!("server executable was not found in the update archive")
}

fn argument_path(args: &[String], name: &str) -> anyhow::Result<PathBuf> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| PathBuf::from(&pair[1]))
        .with_context(|| format!("missing updater argument {name}"))
}

fn platform_executable_name() -> &'static str {
    if cfg!(windows) {
        "pontemesh-server.exe"
    } else {
        "pontemesh-server"
    }
}

fn platform_asset_name(version: &str) -> String {
    let target = if cfg!(target_os = "windows") {
        "windows-x64.zip"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "macos-arm64.tar.gz"
    } else if cfg!(target_os = "macos") {
        "macos-x64.tar.gz"
    } else {
        "linux-x64.tar.gz"
    };
    format!("pontemesh-server-v{version}-{target}")
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0000_0008 | 0x0000_0200);
}

#[cfg(not(windows))]
fn detach(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_a_platform_specific_release_asset() {
        let name = platform_asset_name("0.3.6");

        assert!(name.starts_with("pontemesh-server-v0.3.6-"));
        assert!(name.ends_with(".zip") || name.ends_with(".tar.gz"));
    }

    #[test]
    fn rejects_an_archive_that_does_not_match_the_manifest() {
        let asset = ManifestAsset {
            name: "artifact".to_owned(),
            size: 3,
            sha256: "invalid".to_owned(),
        };

        assert!(validate_archive(b"abc", &asset).is_err());
    }

    #[test]
    fn validates_manifest_identity_and_asset() {
        let manifest = ReleaseManifest {
            schema: 1,
            product: "server".to_owned(),
            version: "0.3.6".to_owned(),
            assets: vec![ManifestAsset {
                name: platform_asset_name("0.3.6"),
                size: 10,
                sha256: "digest".to_owned(),
            }],
        };

        assert!(validate_manifest(&manifest, "0.3.6", &platform_asset_name("0.3.6")).is_ok());
        assert!(validate_manifest(&manifest, "0.3.7", &platform_asset_name("0.3.6")).is_err());
    }
}
