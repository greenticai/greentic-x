# simple-case-app

Runnable example showing the basic case lifecycle with the real local `gx.case` contract.

Flow:
- install and activate the `gx.case` contract from `contracts/case/contract.json`
- create a case from `contracts/case/examples/case.created.json`
- patch case metadata
- append an evidence reference
- transition the case from `new` to `triaged`

Run with:

```bash
cargo run -p simple-case-app
```
