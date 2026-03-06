# approval-basic Source

Deterministic reference logic:

1. Read `risk_score`.
2. Approve when the score is at or below the local threshold.
3. Return a shaped result with `approved` and a short `reason`.
