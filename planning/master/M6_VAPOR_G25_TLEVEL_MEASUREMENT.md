# M6 Vapor G25: `T_LEVEL.OUT` Boundary-Flux Measurement Report

- **Case:** `vapor_met_bare_soil_on` (2400 steps, constant `dt = 0.1 h`, `t = 0.1..240.0 h`, `KodTop` switches at `42.7`, `120.1`, `200.3 h`)
- **Sidecar:** `reference/vapor_fortran/fixtures/vapor_met_bare_soil_on_tlevel.json`
- **Pinned Sidecar SHA-256:** `4f819abe86fe0cfd33b9b82e04f30da1bb2f78d8530560ce1ebc3079dbfff20b`
- **Rust implementation:** `hydrus_core::heat::vapor::output::t_level_fluxes_for_step` (Plan 33, task 33.1.3 `a8bc3fc`, wired in 33.2.2 `7a1fec8`)
- **Measurement harness:** `crates/hydrus-core/tests/vapor_met_tlevel_parity.rs`
- **Command:** `cargo test -p hydrus-core --release --test vapor_met_tlevel_parity -- --nocapture`
- **Gate:** Input to Plan 33 Gate **G-1** (user bounds approval for `criteria.json` v8).

## Observed Sign Convention

Both Rust (`StepTrace::v_top_reported`, `StepTrace::v_bot_reported`) and Fortran (`T_LEVEL.OUT` `vTop`, `vBot` via `TLInf` at `OUTPUT.FOR:25-54`) use **positive upward** (out of the soil at the top boundary, into the soil at the bottom boundary):

- Evaporation (`r_top > 0`, e.g., `t = 42.7 h`, `200.3 h`) yields `vTop > 0` (`+2.1374e-2`, `+2.3698e-2 cm/h`).
- Rain (`r_top = -9.1341e-2 cm/h`, `t = 120.1..144.0 h`) yields `vTop < 0` (`-9.1435e-2 cm/h`).
- Bottom drainage yields `vBot < 0` (`-8.863e-3 cm/h`); upward bottom inflow yields `vBot > 0` (`+4.333e-3 cm/h`).
- **Sign agreement:** `2400 / 2400` steps match sign for both `vTop` and `vBot` (`0` opposite-sign steps in either mode).
- **Systematic error check:** None. No sign flip, constant offset, or scale factor error is present.

## Per-Step Discrepancies (cm/h)

The Fortran `T_LEVEL.OUT` format 170 prints fluxes in `e13.5` (5 significant digits), giving a rounding floor of half a unit in the 5th significant digit (`0.5 * 10^(floor(log10(|x|)) - 4)`): `5.0e-7 cm/h` at `|x| ~ 1e-2..9e-2 cm/h` and `5.0e-8 cm/h` at `|x| ~ 1e-3..9e-3 cm/h`.

| Metric | Mode | Steps | Max `|Δ|` (cm/h) | At `t` (h) | Max rel | At `t` (h) | Max `e13.5` floor | Max `|Δ|/floor` | Sign same/opp |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `vTop` (`TLInf`) | Flux (`kod < 0`) | 1228 | `5.150e-7` | 169.0 | `2.183e-5` | 169.0 | `5.000e-7` | 1.03 | 1228 / 0 |
| `vTop` (`TLInf`) | Head (`kod > 0`) | 1172 | `5.402e-7` | 227.9 | `5.125e-5` | 227.9 | `5.000e-7` | 1.59 | 1172 / 0 |
| `vBot` (`TLInf`) | Flux (`kod < 0`) | 1228 | `1.113e-6` | 0.5 | `8.400e-3` | 148.9 | `5.000e-7` | 373.64 | 1228 / 0 |
| `vBot` (`TLInf`) | Head (`kod > 0`) | 1172 | `2.301e-7` | 115.6 | `7.951e-3` | 57.6 | `5.000e-8` | 995.32 | 1172 / 0 |
| `vTop` (switching ref) | Flux (`kod < 0`) | 1228 | `8.009e-5` | 120.1 | `8.760e-4` | 120.1 | `5.000e-7` | 160.19 | 1228 / 0 |
| `vTop` (switching ref) | Head (`kod > 0`) | 1172 | `3.339e-5` | 48.1 | `3.303e-3` | 96.1 | `5.000e-7` | 601.51 | 1172 / 0 |

Peak `|Fortran|` magnitudes: `|vTop| = 9.1435e-2 cm/h` (`e13.5` floor `5.0e-7`), `|vBot| = 1.1799e-2 cm/h` (`e13.5` floor `5.0e-7`).

Notes:
- `vTop` matches Fortran to within `1.59x` the `e13.5` print floor across all 2400 steps (`max |ΔvTop| = 5.40e-7 cm/h`).
- `vBot` has `max |ΔvBot| = 1.11e-6 cm/h` at `t = 0.5 h` (single-precision `f32` cancellation in Fortran's bottom half-cell storage term during the initial transient). The `~8e-3` relative maxima at `t = 57.6 h` and `148.9 h` occur near `vBot ≈ 0` zero crossings where `|ΔvBot|` is small.
- Omitting the vapor storage term (the switching-form `StepTrace::v_top` reference) increases `max |ΔvTop|` by ~150x (`8.01e-5 cm/h`), confirming that the `TLInf` vapor storage term is essential.

## Worst 10 Steps

### `|ΔvTop|`

| Rank | `t_level` | `t` (h) | `kod` | Rust `vTop` | Fortran `vTop` | `|ΔvTop|` | Rel | `e13.5` floor |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 2279 | 227.9 | 4 | `+1.053954e-2` | `+1.053900e-2` | `5.402e-7` | `5.125e-5` | `5.000e-7` |
| 2 | 2199 | 219.9 | 4 | `+1.174753e-2` | `+1.174700e-2` | `5.333e-7` | `4.540e-5` | `5.000e-7` |
| 3 | 599 | 59.9 | 4 | `+1.185853e-2` | `+1.185800e-2` | `5.277e-7` | `4.450e-5` | `5.000e-7` |
| 4 | 646 | 64.6 | 4 | `+1.107753e-2` | `+1.107700e-2` | `5.277e-7` | `4.764e-5` | `5.000e-7` |
| 5 | 2014 | 201.4 | 4 | `+1.938152e-2` | `+1.938100e-2` | `5.233e-7` | `2.700e-5` | `5.000e-7` |
| 6 | 541 | 54.1 | 4 | `+1.312752e-2` | `+1.312700e-2` | `5.168e-7` | `3.937e-5` | `5.000e-7` |
| 7 | 675 | 67.5 | 4 | `+1.066952e-2` | `+1.066900e-2` | `5.157e-7` | `4.833e-5` | `5.000e-7` |
| 8 | 1690 | 169.0 | -4 | `+2.358849e-2` | `+2.358900e-2` | `5.150e-7` | `2.183e-5` | `5.000e-7` |
| 9 | 1932 | 193.2 | -4 | `+2.526251e-2` | `+2.526200e-2` | `5.135e-7` | `2.033e-5` | `5.000e-7` |
| 10 | 1919 | 191.9 | -4 | `+2.358851e-2` | `+2.358800e-2` | `5.120e-7` | `2.170e-5` | `5.000e-7` |

### `|ΔvBot|`

| Rank | `t_level` | `t` (h) | `kod` | Rust `vBot` | Fortran `vBot` | `|ΔvBot|` | Rel | `e13.5` floor |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 5 | 0.5 | -4 | `-8.863187e-3` | `-8.864300e-3` | `1.113e-6` | `1.256e-4` | `5.000e-8` |
| 2 | 10 | 1.0 | -4 | `-8.863451e-3` | `-8.864300e-3` | `8.494e-7` | `9.582e-5` | `5.000e-8` |
| 3 | 17 | 1.7 | -4 | `-8.863737e-3` | `-8.863000e-3` | `7.372e-7` | `8.318e-5` | `5.000e-8` |
| 4 | 14 | 1.4 | -4 | `-8.863622e-3` | `-8.864300e-3` | `6.782e-7` | `7.651e-5` | `5.000e-8` |
| 5 | 15 | 1.5 | -4 | `-8.863661e-3` | `-8.863000e-3` | `6.613e-7` | `7.461e-5` | `5.000e-8` |
| 6 | 20 | 2.0 | -4 | `-8.863845e-3` | `-8.863200e-3` | `6.452e-7` | `7.279e-5` | `5.000e-8` |
| 7 | 12 | 1.2 | -4 | `-8.863539e-3` | `-8.862900e-3` | `6.392e-7` | `7.212e-5` | `5.000e-8` |
| 8 | 13 | 1.3 | -4 | `-8.863581e-3` | `-8.863000e-3` | `5.811e-7` | `6.557e-5` | `5.000e-8` |
| 9 | 1729 | 172.9 | -4 | `-1.029055e-2` | `-1.029000e-2` | `5.482e-7` | `5.328e-5` | `5.000e-7` |
| 10 | 18 | 1.8 | -4 | `-8.863774e-3` | `-8.864300e-3` | `5.261e-7` | `5.935e-5` | `5.000e-8` |

## Switch Steps and Rain Record

### Switch steps (`42.7`, `120.1`, `200.3 h`)

| `t` (h) | `kod` | Rust `vTop` | Fortran `vTop` | `|ΔvTop|` | Rust `vBot` | Fortran `vBot` | `|ΔvBot|` | Rust `vTop` (switching) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 42.7 | 4 | `+2.137401e-2` | `+2.137400e-2` | `8.282e-9` | `-2.461696e-3` | `-2.461700e-3` | `3.880e-9` | `+2.137385e-2` |
| 120.1 | -4 | `-9.143548e-2` | `-9.143500e-2` | `4.839e-7` | `+4.332915e-3` | `+4.332900e-3` | `1.499e-8` | `-9.135491e-2` |
| 200.3 | 4 | `+2.369839e-2` | `+2.369800e-2` | `3.863e-7` | `-2.965233e-3` | `-2.965200e-3` | `3.255e-8` | `+2.369834e-2` |

### Rain record (`r_top = -9.1341e-2 cm/h`, 240 steps, `t = 120.1..144.0 h`)

| `t` (h) | `kod` | `r_top` | Rust `vTop` | Fortran `vTop` | `|ΔvTop|` | Rust `vBot` | Fortran `vBot` | `|ΔvBot|` |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 120.1 (first & worst) | -4 | `-9.134100e-2` | `-9.143548e-2` | `-9.143500e-2` | `4.839e-7` | `+4.332915e-3` | `+4.332900e-3` | `1.499e-8` |
| 144.0 (last) | -4 | `-9.134100e-2` | `-9.134124e-2` | `-9.134100e-2` | `2.392e-7` | `+3.675401e-3` | `+3.675400e-3` | `6.190e-10` |

## Cumulative Fluxes (cm)

| Quantity | Final Rust (cm) | Final Fortran (cm) | Final `|Δ|` (cm) | Final rel | Final `e13.5` floor | Max `|Δ|` over run (cm) | At `t` (h) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `sum_v_top` (`ΣvTop·dt`) | `+1.334075e0` | `+1.334100e0` | `2.549e-5` | `1.911e-5` | `5.000e-5` | `5.385e-5` | 51.3 |
| `sum_v_bot` (`ΣvBot·dt`) | `-3.978501e-1` | `-3.978500e-1` | `1.057e-7` | `2.656e-7` | `5.000e-6` | `8.255e-6` | 234.5 |

## Bound Options for Gate G-1 (`criteria.json` v8)

`criteria.json` `checks` entries use `{absolute, relative, scale, unit, assumptions}` with tolerance `tol = max(absolute, relative * |ref|)` (or `scale`-based floor). Because `vBot` and `sum_v_top` cross zero, an absolute floor is required alongside any relative term.

### Per-step boundary flux (`t_level_boundary_flux`, unit `cm/h`)
Observed maxima: `max |ΔvTop| = 5.40e-7 cm/h`, `max |ΔvBot| = 1.11e-6 cm/h`, `e13.5` floor at peak = `5.0e-7 cm/h`.

| Option | `absolute` (cm/h) | `relative` | `scale` | Pass margin (`vTop` / `vBot`) | Notes |
| --- | ---: | ---: | --- | --- | --- |
| **A (Tight)** | `2.0e-6` | `1.0e-4` | `ref_abs` | `3.7x` / `1.8x` | Close to observed `f32` cancellation in `vBot` at `t=0.5 h` (`1.11e-6`). |
| **B (Balanced — Recommended)** | `5.0e-6` | `2.0e-4` | `ref_abs` | `9.3x` / `4.5x` | `10x` the `e13.5` print floor; cleanly separates `TLInf` (`1.11e-6`) from switching-form `v_top` (`8.01e-5`, fails by `16x`). |
| **C (Conservative)** | `1.0e-5` | `5.0e-4` | `ref_abs` | `18.5x` / `9.0x` | Still rejects switching-form `v_top` (`8.01e-5`, fails by `8x`) with wider headroom. |

### Cumulative boundary flux (`t_level_cumulative_flux`, unit `cm`)
Observed maxima: `max |Δsum_v_top| = 5.39e-5 cm` (final `2.55e-5 cm`, `e13.5` floor `5.0e-5 cm`), `max |Δsum_v_bot| = 8.25e-6 cm` (final `1.06e-7 cm`, `e13.5` floor `5.0e-6 cm`).

| Option | `absolute` (cm) | `relative` | `scale` | Pass margin (`sum_v_top` / `sum_v_bot`) | Notes |
| --- | ---: | ---: | --- | --- | --- |
| **A (Tight)** | `1.0e-4` | `1.0e-4` | `ref_abs` | `1.9x` / `12.1x` | Only `2x` the `e13.5` print floor (`5.0e-5 cm`). |
| **B (Balanced — Recommended)** | `2.5e-4` | `2.0e-4` | `ref_abs` | `4.6x` / `30.3x` | `5x` the `e13.5` print floor; catches ~0.02% cumulative drift. |
| **C (Conservative)** | `5.0e-4` | `5.0e-4` | `ref_abs` | `9.3x` / `60.6x` | `10x` the `e13.5` print floor. |

**Recommendation:** Adopt **Option B** for both checks (`t_level_boundary_flux`: `absolute = 5.0e-6 cm/h`, `relative = 2.0e-4`, `scale = "ref_abs"`; `t_level_cumulative_flux`: `absolute = 2.5e-4 cm`, `relative = 2.0e-4`, `scale = "ref_abs"`). Both give comfortable headroom above the `e13.5` print floor and Fortran `f32` cancellation while decisively rejecting the switching-form `v_top` omission (`8.01e-5 cm/h`).

## Raw Test Output

```text
running 2 tests
test sidecar_structure_and_time_alignment ... ok
=== Per-step summary (cm/h) ===
vTop (flux kod<0)      n=1228 | max_abs=5.149777e-7 (t=169.0, R=+2.358849e-2, F=+2.358900e-2) | max_rel=2.183127e-5 (t=169.0) | max_floor=5.000e-7 | max(|d|/floor)=1.03 (t=169.0) | sign same/opp=1228/0
vTop (head kod>0)      n=1172 | max_abs=5.401705e-7 (t=227.9, R=+1.053954e-2, F=+1.053900e-2) | max_rel=5.125443e-5 (t=227.9) | max_floor=5.000e-7 | max(|d|/floor)=1.59 (t=106.1) | sign same/opp=1172/0
vBot (flux kod<0)      n=1228 | max_abs=1.113296e-6 (t=  0.5, R=-8.863187e-3, F=-8.864300e-3) | max_rel=8.400466e-3 (t=148.9) | max_floor=5.000e-7 | max(|d|/floor)=373.64 (t=148.9) | sign same/opp=1228/0
vBot (head kod>0)      n=1172 | max_abs=2.301089e-7 (t=115.6, R=+4.198070e-3, F=+4.198300e-3) | max_rel=7.951247e-3 (t= 57.6) | max_floor=5.000e-8 | max(|d|/floor)=995.32 (t= 57.6) | sign same/opp=1172/0
=== Reference: switching-form StepTrace::v_top vs Fortran vTop (cm/h) ===
vTop_sw (flux kod<0)   n=1228 | max_abs=8.009369e-5 (t=120.1, R=-9.135491e-2, F=-9.143500e-2) | max_rel=8.759632e-4 (t=120.1) | max_floor=5.000e-7 | max(|d|/floor)=160.19 (t=120.1) | sign same/opp=1228/0
vTop_sw (head kod>0)   n=1172 | max_abs=3.338886e-5 (t= 48.1, R=+1.432661e-2, F=+1.436000e-2) | max_rel=3.303463e-3 (t= 96.1) | max_floor=5.000e-7 | max(|d|/floor)=601.51 (t= 96.1) | sign same/opp=1172/0
Peak |Fortran|: vTop=9.143500e-2 (e13.5 floor=5.000e-7), vBot=1.179900e-2 (e13.5 floor=5.000e-7)

=== Worst 10 steps: |ΔvTop| ===
rank  t_level    time  kod        rust_vTop     fortran_vTop       |ΔvTop|         rel    e13.5_fl
   1     2279   227.9    4  +1.053954e-2  +1.053900e-2  5.401705e-7  5.1254e-5  5.000e-7
   2     2199   219.9    4  +1.174753e-2  +1.174700e-2  5.332596e-7  4.5395e-5  5.000e-7
   3      599    59.9    4  +1.185853e-2  +1.185800e-2  5.277280e-7  4.4504e-5  5.000e-7
   4      646    64.6    4  +1.107753e-2  +1.107700e-2  5.276550e-7  4.7635e-5  5.000e-7
   5     2014   201.4    4  +1.938152e-2  +1.938100e-2  5.233397e-7  2.7003e-5  5.000e-7
   6      541    54.1    4  +1.312752e-2  +1.312700e-2  5.168107e-7  3.9370e-5  5.000e-7
   7      675    67.5    4  +1.066952e-2  +1.066900e-2  5.156699e-7  4.8333e-5  5.000e-7
   8     1690   169.0   -4  +2.358849e-2  +2.358900e-2  5.149777e-7  2.1831e-5  5.000e-7
   9     1932   193.2   -4  +2.526251e-2  +2.526200e-2  5.134893e-7  2.0327e-5  5.000e-7
  10     1919   191.9   -4  +2.358851e-2  +2.358800e-2  5.119534e-7  2.1704e-5  5.000e-7

=== Worst 10 steps: |ΔvBot| ===
rank  t_level    time  kod        rust_vBot     fortran_vBot       |ΔvBot|         rel    e13.5_fl
   1        5     0.5   -4  -8.863187e-3  -8.864300e-3  1.113296e-6  1.2559e-4  5.000e-8
   2       10     1.0   -4  -8.863451e-3  -8.864300e-3  8.493869e-7  9.5821e-5  5.000e-8
   3       17     1.7   -4  -8.863737e-3  -8.863000e-3  7.371888e-7  8.3176e-5  5.000e-8
   4       14     1.4   -4  -8.863622e-3  -8.864300e-3  6.782209e-7  7.6512e-5  5.000e-8
   5       15     1.5   -4  -8.863661e-3  -8.863000e-3  6.612509e-7  7.4608e-5  5.000e-8
   6       20     2.0   -4  -8.863845e-3  -8.863200e-3  6.451681e-7  7.2792e-5  5.000e-8
   7       12     1.2   -4  -8.863539e-3  -8.862900e-3  6.391892e-7  7.2120e-5  5.000e-8
   8       13     1.3   -4  -8.863581e-3  -8.863000e-3  5.811407e-7  6.5569e-5  5.000e-8
   9     1729   172.9   -4  -1.029055e-2  -1.029000e-2  5.482303e-7  5.3278e-5  5.000e-7
  10       18     1.8   -4  -8.863774e-3  -8.864300e-3  5.261274e-7  5.9354e-5  5.000e-8

=== Switch steps (42.7, 120.1, 200.3 h) ===
  time  kod        rust_vTop     fortran_vTop       |ΔvTop|        rust_vBot     fortran_vBot       |ΔvBot|    rust_vTop_sw
  42.7    4  +2.137401e-2  +2.137400e-2  8.281548e-9  -2.461696e-3  -2.461700e-3  3.880300e-9  +2.137385e-2
 120.1   -4  -9.143548e-2  -9.143500e-2  4.839152e-7  +4.332915e-3  +4.332900e-3  1.499137e-8  -9.135491e-2
 200.3    4  +2.369839e-2  +2.369800e-2  3.863236e-7  -2.965233e-3  -2.965200e-3  3.255341e-8  +2.369834e-2

=== Rain steps (Fortran r_top < 0, count=240) ===
  time  kod         r_top        rust_vTop     fortran_vTop       |ΔvTop|        rust_vBot     fortran_vBot       |ΔvBot|
 120.1   -4  -9.134100e-2  -9.143548e-2  -9.143500e-2  4.839152e-7  +4.332915e-3  +4.332900e-3  1.499137e-8
 144.0   -4  -9.134100e-2  -9.134124e-2  -9.134100e-2  2.392198e-7  +3.675401e-3  +3.675400e-3  6.189800e-10

=== Cumulative fluxes (cm) ===
sum_v_top: final Rust=+1.334075e0, Fortran=+1.334100e0, |Δ|=2.549436e-5, rel=1.910978e-5, e13.5_fl=5.000e-5 | max_over_run |Δ|=5.385135e-5 (t= 51.3), max_rel=1.843827e-3
sum_v_bot: final Rust=-3.978501e-1, Fortran=-3.978500e-1, |Δ|=1.056879e-7, rel=2.656476e-7, e13.5_fl=5.000e-6 | max_over_run |Δ|=8.254635e-6 (t=234.5), max_rel=3.214673e-4
test measure_tlevel_flux_discrepancies ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
```
