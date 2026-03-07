# playbook-select Source

Deterministic reference logic:

1. Inspect simple context fields such as `severity` and `signal_type`.
2. Choose a playbook identifier and route label.
3. Return the selected playbook without side effects.
