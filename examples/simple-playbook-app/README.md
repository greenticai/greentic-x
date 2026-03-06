# simple-playbook-app

Runnable example showing playbook selection, playbook-run tracking, and outcome updates with the real local contracts and ops.

Flow:
- install and activate the `gx.playbook` and `gx.outcome` contracts
- install the `playbook-select` op from `ops/playbook-select/op.json`
- create a playbook resource
- invoke the selector op to choose a route
- create and update a `playbook-run`
- append a step result
- create and transition an `outcome`

Run with:

```bash
cargo run -p simple-playbook-app
```
