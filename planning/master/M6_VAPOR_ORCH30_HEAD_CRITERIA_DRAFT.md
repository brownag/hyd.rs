# M6 vapor ORCH30 head discrimination: options (DRAFT, not written)

Status: DRAFT for review. Nothing here is written to `reference/vapor_fortran/criteria.json`.
No tolerance is changed by this document.

## Problem
`free_drain_on_off_head` fails at factor 2.0. The on/off head difference is 3.215 cm at 240 h (top node, index 0). The comparator tolerance is `0.5 + 0.02·|head_on|`, which is about 3.5 cm at −151 cm. The largest passing factor is r_max = 0.9121.

Cause (see STATE_30.md investigation note): the on/off head effect comes from the thermal vapor term in the water equation (`WATFLOW.FOR` lines 226, 259, 262, 289). It is the designed coupling, not a defect. Turning those terms off makes vapor-on heads equal vapor-off heads.

## Options
| Option | Change | Pros | Cons |
|---|---|---|---|
| A (recommended) | Do not write a head discrimination entry for free drainage. Keep the liquid_moisture and temperature_c free-drainage entries (both pass). Head is covered by the exact fixture comparison `free_drainage_{on,off}_matches_fixture`. | No threshold change. No tolerance change. Head behavior is still checked against Fortran. | No head-specific discrimination signal for free drainage. |
| B | Keep the head entry at factor 2.0 but use an absolute-only head threshold for discrimination. | Free-drainage on/off (3.215 cm) passes. | Changes the comparator's tolerance form. Wet closed pair head (0.897 cm) still fails at 1.0 cm. Needs an explicit tolerance decision. |
| C | Keep the head entry at a factor below 0.9121. | Minimal change. | Weak check, and an exception to the factor 2.0 used elsewhere. |

## Evidence
- `free_drain_on_off_head`: r_max 0.9121 at t=240 h, node 1 (`.development/30.6.1/disc.json`, `planning/tasks/30.6.1.report.md` r_max table).
- Counterfactual with thermal vapor terms off: vapor-on heads equal vapor-off heads (to 0.001 cm) at all nodes and times (`.development/30.7.1/`).

## Decision needed
Choose A, B or C. Under option A, the reviewer confirms that the head check stays with the exact fixture comparison and no discrimination entry is added.
