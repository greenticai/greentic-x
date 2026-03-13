#![allow(dead_code)]

use std::path::Path;

use greentic_distributor_client::{DistClient, DistOptions};
use tokio::runtime::Runtime;

pub(crate) struct ResolvedRemoteRef {
    pub(crate) resolved_digest: String,
}

pub(crate) trait RemoteRefResolver {
    fn resolve(&self, cache_root: &Path, reference: &str) -> Result<ResolvedRemoteRef, String>;
}

pub(crate) struct DistributorRemoteRefResolver;

impl RemoteRefResolver for DistributorRemoteRefResolver {
    fn resolve(&self, cache_root: &Path, reference: &str) -> Result<ResolvedRemoteRef, String> {
        let options = DistOptions {
            allow_tags: true,
            offline: false,
            cache_dir: cache_root.join(".gx").join("cache").join("distributor"),
            ..DistOptions::default()
        };
        let runtime =
            Runtime::new().map_err(|err| format!("failed to start distributor runtime: {err}"))?;
        let resolved = runtime
            .block_on(DistClient::new(options).resolve_ref(reference))
            .map_err(|err| format!("failed to resolve remote source ref {reference}: {err}"))?;
        Ok(ResolvedRemoteRef {
            resolved_digest: resolved.resolved_digest,
        })
    }
}

pub(crate) fn is_resolvable_remote_source_ref(value: &str) -> bool {
    value.starts_with("oci://") || value.starts_with("repo://") || value.starts_with("store://")
}

pub(crate) fn pin_reference_to_digest(reference: &str, digest: &str) -> Option<String> {
    if reference.contains('@') {
        return Some(reference.to_owned());
    }
    if !reference.contains(":latest") {
        return None;
    }
    let digest = normalize_digest(digest)?;
    let (prefix, _) = reference.rsplit_once(":latest")?;
    Some(format!("{prefix}@{digest}"))
}

fn normalize_digest(digest: &str) -> Option<String> {
    let trimmed = digest.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("sha256:") {
        Some(trimmed.to_owned())
    } else {
        Some(format!("sha256:{trimmed}"))
    }
}

#[cfg(test)]
mod tests {
    use super::pin_reference_to_digest;

    #[test]
    fn pin_reference_rewrites_latest_tag() {
        let pinned =
            pin_reference_to_digest("oci://ghcr.io/demo/assistant:latest", "sha256:abc123")
                .expect("pinned ref");
        assert_eq!(pinned, "oci://ghcr.io/demo/assistant@sha256:abc123");
    }

    #[test]
    fn pin_reference_preserves_existing_digest() {
        let pinned = pin_reference_to_digest("repo://greentic/demo@sha256:def456", "sha256:abc123")
            .expect("existing digest should be preserved");
        assert_eq!(pinned, "repo://greentic/demo@sha256:def456");
    }
}
