# Catalog Repos

GX solution catalogs let downstream solution repos publish a canonical
`catalog.json` that `gx wizard` can consume locally or through OCI.

## Standard Layout

```text
<repo>/
  catalog.json
  assistant_templates/
  bundles/
  views/
  overlays/
  setup_profiles/
  contracts/
  resolvers/
  adapters/
  analysis/
  playbooks/
```

Only the directories you use need to exist. `catalog.json` is always the root
entrypoint.

## Commands

Initialize a new catalog repo:

```bash
gx catalog init zain-x
```

Build or refresh the canonical root catalog:

```bash
gx catalog build --repo zain-x
gx catalog build --repo zain-x --check
```

Validate the root catalog and all referenced assets:

```bash
gx catalog validate --repo zain-x
```

## Wizard Consumption

`greentic-x wizard` always loads the built-in GX catalog, then merges any explicit
catalog sources passed with `--catalog`.

Examples:

```bash
greentic-x wizard --catalog catalog.json
greentic-x wizard --catalog oci://ghcr.io/greenticai/catalogs/zain-x/catalog.json:latest
greentic-x \
  --catalog oci://ghcr.io/greenticai/catalogs/zain-x/catalog.json:latest \
  --catalog oci://ghcr.io/greenticai/catalogs/meeza-x/catalog.json:latest
```

Remote OCI catalogs are fetched through `greentic-distributor-client`. Moving
refs such as `:latest` are resolved with the default `update_then_pin` policy
so generated downstream artifacts can record pinned references.
