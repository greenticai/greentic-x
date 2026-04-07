use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;

use super::catalog::RemoteCatalogFetcher;

pub(crate) fn materialize_bundle_member(
    cwd: &Path,
    bundle_ref: &str,
    member_path: &str,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<PathBuf, String> {
    let bundle_root = materialize_bundle(cwd, bundle_ref, fetcher)?;
    resolve_bundle_member_path(&bundle_root, member_path)
}

fn materialize_bundle(
    cwd: &Path,
    bundle_ref: &str,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<PathBuf, String> {
    if let Some(local_path) = resolve_local_path(cwd, bundle_ref) {
        let bytes = fs::read(&local_path).map_err(|err| {
            format!(
                "failed to read local bundle {}: {err}",
                local_path.display()
            )
        })?;
        let digest = digest_for_bytes(&bytes);
        let bundle_dir = bundle_cache_dir(cwd, &digest);
        unpack_bundle_bytes(&bundle_dir, &bytes, None)?;
        return Ok(bundle_dir);
    }

    let fetched = fetcher.fetch_pack_artifact(cwd, bundle_ref)?;
    let bundle_dir = bundle_cache_dir(cwd, &fetched.resolved_digest);
    if bundle_dir.join(".bundle-unpacked").exists() {
        return Ok(bundle_dir);
    }
    let bytes = fs::read(&fetched.path).map_err(|err| {
        format!(
            "failed to read fetched bundle artifact {}: {err}",
            fetched.path.display()
        )
    })?;
    unpack_bundle_bytes(&bundle_dir, &bytes, Some(&fetched.media_type))?;
    Ok(bundle_dir)
}

fn unpack_bundle_bytes(
    target_dir: &Path,
    bytes: &[u8],
    media_type: Option<&str>,
) -> Result<(), String> {
    if target_dir.join(".bundle-unpacked").exists() {
        return Ok(());
    }
    if target_dir.exists() {
        fs::remove_dir_all(target_dir).map_err(|err| {
            format!(
                "failed to reset bundle cache {}: {err}",
                target_dir.display()
            )
        })?;
    }
    fs::create_dir_all(target_dir).map_err(|err| {
        format!(
            "failed to create bundle cache {}: {err}",
            target_dir.display()
        )
    })?;

    let is_gzip = media_type.is_some_and(|value| value.contains("tar+gzip"))
        || bytes.starts_with(&[0x1f, 0x8b]);

    if is_gzip {
        let decoder = GzDecoder::new(Cursor::new(bytes));
        unpack_tar_archive(target_dir, decoder)?;
    } else {
        unpack_tar_archive(target_dir, Cursor::new(bytes))?;
    }

    fs::write(target_dir.join(".bundle-unpacked"), b"ok").map_err(|err| {
        format!(
            "failed to write bundle cache marker {}: {err}",
            target_dir.display()
        )
    })?;
    Ok(())
}

fn unpack_tar_archive(target_dir: &Path, reader: impl Read) -> Result<(), String> {
    let mut archive = Archive::new(reader);
    let entries = archive
        .entries()
        .map_err(|err| format!("failed to inspect bundle archive: {err}"))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|err| format!("failed to read bundle archive entry: {err}"))?;
        entry.unpack_in(target_dir).map_err(|err| {
            format!(
                "failed to unpack bundle archive into {}: {err}",
                target_dir.display()
            )
        })?;
    }
    Ok(())
}

fn resolve_bundle_member_path(bundle_root: &Path, member_path: &str) -> Result<PathBuf, String> {
    let direct = bundle_root.join(member_path);
    if direct.exists() {
        return Ok(direct);
    }

    let mut child_dirs = fs::read_dir(bundle_root)
        .map_err(|err| {
            format!(
                "failed to read extracted bundle {}: {err}",
                bundle_root.display()
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    child_dirs.sort();

    if child_dirs.len() == 1 {
        let nested = child_dirs[0].join(member_path);
        if nested.exists() {
            return Ok(nested);
        }
    }

    Err(format!(
        "bundle member {member_path} was not found in extracted bundle {}",
        bundle_root.display()
    ))
}

fn bundle_cache_dir(cwd: &Path, digest: &str) -> PathBuf {
    cwd.join(".gx")
        .join("cache")
        .join("bundles")
        .join(trim_digest_prefix(digest))
}

fn trim_digest_prefix(digest: &str) -> &str {
    digest
        .strip_prefix("sha256:")
        .unwrap_or_else(|| digest.trim_start_matches('@'))
}

fn digest_for_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

fn resolve_local_path(cwd: &Path, reference: &str) -> Option<PathBuf> {
    let path = Path::new(reference);
    if path.is_absolute() && path.exists() {
        return Some(path.to_path_buf());
    }
    let candidate = cwd.join(reference);
    candidate.exists().then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wizard::catalog::{FetchResult, RemoteCatalogFetcher, ResolvedPackArtifact};
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tempfile::TempDir;

    struct StubFetcher {
        artifact: Vec<u8>,
    }

    impl RemoteCatalogFetcher for StubFetcher {
        fn fetch_json(&self, _cache_root: &Path, _reference: &str) -> Result<FetchResult, String> {
            Err("unused".to_owned())
        }

        fn resolve_pack_ref(&self, _cache_root: &Path, _reference: &str) -> Result<String, String> {
            Err("unused".to_owned())
        }

        fn fetch_pack_artifact(
            &self,
            cache_root: &Path,
            _reference: &str,
        ) -> Result<ResolvedPackArtifact, String> {
            let path = cache_root.join(".gx/test-bundle.tar.gz");
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            fs::write(&path, &self.artifact).map_err(|err| err.to_string())?;
            Ok(ResolvedPackArtifact {
                path,
                resolved_digest: "sha256:testbundle".to_owned(),
                media_type: "application/vnd.oci.image.layer.v1.tar+gzip".to_owned(),
            })
        }
    }

    #[test]
    fn materializes_bundle_member_from_remote_archive() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let bundle_bytes = gzipped_bundle(&[
            (
                "assistant_templates/network.json",
                br#"{"ok":true}"#.as_slice(),
            ),
            (
                "catalog.json",
                br#"{"schema":"gx.catalog.index.v1"}"#.as_slice(),
            ),
        ])?;
        let path = materialize_bundle_member(
            temp.path(),
            "store://ghcr.io/demo/zain-x-bundle:latest",
            "assistant_templates/network.json",
            &StubFetcher {
                artifact: bundle_bytes,
            },
        )?;
        assert_eq!(fs::read_to_string(path)?, r#"{"ok":true}"#);
        Ok(())
    }

    fn gzipped_bundle(files: &[(&str, &[u8])]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for (path, bytes) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *bytes)?;
        }
        let encoder = builder.into_inner()?;
        Ok(encoder.finish()?)
    }
}
