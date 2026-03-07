# Greentic-X Packs

This directory contains wizard-backed `greentic-pack` source packs for the
current Greentic-X reference material.

- `greentic-x-contracts-reference`: bundles the current reference contracts from
  `contracts/` into a contract-oriented pack scaffold.
- `greentic-x-ops-reference`: bundles the current reference ops from `ops/`
  into an ops-oriented pack scaffold.
- `greentic-x-runtime-capability-reference`: provides a runtime-capability pack
  scaffold with a placeholder component and an example capability offer.

The initial scaffolds were created with:

```bash
greentic-pack wizard apply --answers packs/_wizard/<name>.answers.json
```

The checked-in packs are source directories, not prebuilt archives. Build and
inspect them with:

```bash
greentic-pack build --in packs/greentic-x-contracts-reference
greentic-pack build --in packs/greentic-x-ops-reference
greentic-pack build --in packs/greentic-x-runtime-capability-reference
```

The generated component bundles are placeholders. The current packs focus on
pack shape, bundled assets, and extension metadata rather than production
runtime execution.
