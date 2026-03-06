# end-to-end-demo

Runnable end-to-end demo covering the main Greentic-X building blocks.

Flow:
- install and activate the local case, evidence, outcome, and playbook contracts
- install the local `playbook-select`, `rca-basic`, and `approval-basic` ops
- create a case and select a playbook
- create and run a playbook-run
- create evidence and attach it to the case
- invoke RCA and approval ops
- create and transition an outcome
- complete the playbook-run and resolve the case

Run with:

```bash
cargo run -p end-to-end-demo
```
