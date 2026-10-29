# Solute convergence evidence, Orchestration 23 (task 23.6.1)

Approval: `planning/master/M5V_CLASSIFY_APPROVAL_23.md` (commit 9f3ca9f), signed by the user.
Campaign root: `23-classify-3`. Cases: `reference/solute_verification/CASES_V2.json`, SHA-256 `ceddd59d...8cab7`.

## Status

- 23.4.1, 23.4.2, 23.4.4: executed and verified, exit 0.
- 23.4.3: all six solver runs completed, but the batch failed at analysis with
  `co-refined study axes`. The batch was not verified and has no classification.
- Scientific accuracy is not accepted, thresholds are unassigned, and M5-V stays open.

## Batches

| Worktree | Rows | Execute exit | Verify exit | Batch |
| --- | --- | --- | --- | --- |
| 23.4.1 | diffusion space 65/97/129; time 97 at dt 2e-6, 2e-8 | 0 | 0 | `batch-cb0b930e20e84b14899146d265e58fd4` |
| 23.4.2 | scalar_robin space 129/193/257; time 193 at dt 2e-6, 2e-8 | 0 | 0 | `batch-b021c02566844a4f970c52887eb0a4c0` |
| 23.4.3 | example3 space 251/501/1001 at dt 0.025; 501/751/1001 at dt 0.04 | 2 | not run | `none` |
| 23.4.4 | example3 time 751 at dt 0.05, 0.025 | 0 | 0 | `batch-f0bce16661ad4f3bbd9c39d2edeb8319` |

## Bounds and producer hashes

- Bounds SHA-256 changed from `234333512ffee95b...fa32cf` to `8b9f2e3f44b8fc31eab084ad46140e5aae50ef92a5a5ebb37d14840b83e7429c`. Only `producer_sha256` changed. All bound values are unchanged.
- Changed producer files (original to final):

| File | Original SHA-256 | Final SHA-256 |
| --- | --- | --- |
| `tools/run_solute_convergence.py` | `f9e20b0eea24d618d464f5fbd1929de4be6840bdc9a59711baef496eeb3beec6` | `4c2cbded475427b163e52ad7216211b98f9439de89cedcd27c6931bcee744a59` |
| `tools/run_solute_verification_reference.py` | `db1c31a2e596401e8bbd9c344605bdaa73db9894fc2614dc1f8988201286dcba` | `e896a0e23083eceac1a0508f89a6d5fe66ff857237e64a54f69c872bdc9f3b31` |
| `tools/solute_benchmark_lifecycle.py` | `fcb9094e443670a7c0e736158de47ffe5e59f6c938ffefdb9a5be5ed794c0195` | `2a2a482ba2cf72a8ed4a659395e126a380080ef598cbb89b1e8361ea2871708e` |
| `tools/solute_convergence_evidence.py` | `f3bdf78f454508a6457c5fa65cca5cf21f22cb6e606da839157cd92b720f8d63` | `7d936a265e9d0aaf796f765d6df1bc476f08165c5785dd44fcaf8db0b9ef1823` |
| `tools/solute_reference_cache.py` | `1435d3209a578cf13b2210556acac43da2ade6d45faa45c8110af87ea345dc42` | `99ec4e86889a9b0e14b668540cb877208c34f73af3740a182262863aadb8961a` |

## Classification

- 23.4.1, 23.4.2, and 23.4.4 produced classifications, with bounds SHA `8b9f2e3f44b8...`.
- Every finding is `inconclusive` with reason `control_missing`. The plan left out the
  control rows (`space_control`, `time_control`), so no finding can be `achieved` or `failed`.
- 23.4.3: not classified. The analyzer refuses a refinement group that mixes two fixed-axis values.
  The 23.4.3 plan mixes space rows at dt 0.025 with space rows at dt 0.04 in one group.
- Unassigned claims: accuracy thresholds (unassigned by plan); M5-V closure (open);
  every finding class for 23.4.3 (no classification produced).

## Refused and planned folders (left in place)

- `23.4.1/.development/23-classify/` (campaign 23-classify): planned `batch-83e53831...`;
  refused `batch-bb34cad7...`, `batch-bd655447...`, `batch-84ab6f4e...`.
  The first row run failed on the harness allowlist; the row is consumed in that ledger.
- `23.4.1/.development/23-classify-2/`: failed `batch-c80c050f...`. Five runs completed, then
  execute failed on the global duplicate check, which is now scoped. Provenance changed, so resume was refused.
- Planned-only dry-run batches are under each worktree's `.development/23-classify/`.
- `23.4.4/.development/13.4.3/test-default-no-work/`: created by the unit tests.
