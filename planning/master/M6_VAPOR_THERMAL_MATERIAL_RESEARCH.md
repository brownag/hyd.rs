# HYDRUS thermal material parameters: source-backed meaning and limits

## Finding

The official HYDRUS manual identifies Block E `TPar(1..9)`, its units and equations, and approximate phase heat capacities. HYDRUS also documents default thermal-conductivity curves for sand, loam, and clay based on Chung and Horton (1987). This is sufficient for **illustrative order-of-magnitude fixtures**, which the user approved; it is not material-specific calibration or universal validation. The three rows below combine documented textural-class conductivity coefficients with clearly labeled representative solid/organic fractions, dispersivity, and manual-default phase capacities. Their purpose is to establish finite, dimensionally sensible inputs for source behavior tests.

## Authoritative parameter mapping

Manual: [HYDRUS-1D v4.17 Technical Manual (official PC-Progress PDF)](https://www.pc-progress.com/Downloads/Pgm_hydrus1D/HYDRUS1D-4.17.pdf), §4.1, Eqs. (4.2)–(4.8), printed pp. 74–76; Block E, Table 12.5, printed p. xvii (PDF page numbering differs). Table 12.5 explicitly defines:

| TPar row | Official meaning | Unit / use |
|---|---|---|
| 1 | `θn`, volumetric solid fraction | dimensionless |
| 2 | `θo`, volumetric organic matter fraction | dimensionless |
| 3 | longitudinal thermal dispersivity `λL` (the code uses it as `βt`) | length |
| 4–6 | `b1,b2,b3` in `λ0(θ)=b1+b2θ+b3√θ` | conductivity, e.g. W m⁻¹ K⁻¹ |
| 7–9 | `Cn,Co,Cw`, volumetric heat capacities of solid, organic matter, liquid phases | J m⁻³ K⁻¹ |

The requested shorthand `Beta,Qn,Qo,B1,B2,B3,Cn,Co,Cw` is not the manual's literal naming. In the source's default conductivity branch, `TPar(1..3)` are solid fraction, organic fraction, and dispersivity, respectively; `TPar(4..6)` are the three `b` coefficients. The manual calls the first two fractions `θn` and `θo`, not `Qn/Qo`. Do not confuse `TPar(1)` solid fraction with porosity/water content.

Manual Eq. (4.4):

```text
Cp(θ) = Cn θn + Co θo + Cw θ + Ca av
       ≈ (1.92 θn + 2.51 θo + 4.18 θ) × 10^6 J m^-3 °C^-1
```

This supports the phase heat-capacity coefficients `Cn=1.92e6`, `Co=2.51e6`, `Cw=4.18e6 J m^-3 K^-1` as the manual's stated approximation, not as measured values for an unspecified site/material. Eq. (4.7) gives `λ=λ0(θ)+βt Cw |q|`; Eq. (4.8) gives `λ0=b1+b2θ+b3√θ`. The manual calls `b1..b3` empirical and supplies their dimensional units, but this evidence does not prescribe values for a particular soil. Nor is a generic `βt` value supplied by these definitions.

## Source agreement and unit conversion

Pinned source anchors: [`TEMPER.FOR`](../../src_reference/TEMPER.FOR) lines 279–303 implement

```text
Cap = TPar(7)*TPar(1) + TPar(8)*TPar(2) + TPar(9)*θ
Cond = max(0, TPar(4) + TPar(5)*θ + TPar(6)*sqrt(θ))
Cond += TPar(9)*TPar(3)*abs(v)
```

This agrees with the manual's heat capacity and conductivity structure. `Cond` in the default branch is the apparent conductivity, including dispersion. `TEMPER.FOR:189–216` comments vapor heat capacity as `J/m3/K` and latent heat as `J/m3`, then scales them by `1/(xConv*tConv²)`. Vapor sensible capacity is hard-coded as `1.8e6` SI J m⁻³ K⁻¹ (`:192`); the latent term uses `xLatent` (`:204,:213`). `INPUT.FOR:182–202` defines the conversions from metres/seconds: for cm and hours, `xConv=100` and `tConv=1/3600`.

HYDRUS explicitly says to convert thermal capacities using the time unit to the second power and conductivities using it to the third power, in the [official Heat Transport Parameters help](https://www.pc-progress.com/en/OnlineHelp/HYDRUS6/HeatTransportParameters.html). For cm/hour, the factor from SI derived units to the code's cm/hour/kg basis is:

```text
1/(xConv*tConv²) = 1/(100*(1/3600)²) = 129,600
```

The decks use `MUnit=kg`, so no mass rescaling is needed. The vapor routine's hardcoded SI constants are still scaled as in the source; the purpose here is dimensionally consistent fixture material inputs, not asserting calibrated coupling magnitudes.

For conductivity, the default branch is dimensionally consistent with the documented input-unit conversion: `b` has dimensions `M L T^-3 K^-1`, while capacity has `M L^-1 T^-2 K^-1`; the pinned source's Campbell branch converts SI values by `xConv/tConv³` at `TEMPER.FOR:287–300`. With cm/hour this factor is `100/(1/3600)^3 = 4.6656e12`. The default branch uses `b1+b2*theta+b3*sqrt(theta)` directly (`:286`), with the deck coefficients already expressed in the selected project units. The source's added dispersive term is `TPar(9)*TPar(3)*abs(v)` (`:303`); `TPar(3)` is length and velocity is project length/time. Keep `iCampbell=0` and encode the default-class coefficients in the project basis.

## Approved illustrative rows

The official HYDRUS help says default conductivity functions for sand, loam, and clay are provided by HYDRUS based on Chung and Horton (1987). The official HYDRUS Technical Manual, Example 7 Table 9.6, gives loam coefficients `b1=0.243`, `b2=0.393`, `b3=1.534` W m⁻¹ K⁻¹. Saito et al. (2009), Table 2, reports loam `0.244, 0.393, 1.534` and sand `0.228, 2.406, 4.909`. These sources support representative class curves, not every soil. The current project has `iCampbell=0`; use the documented default branch.

For a consistent illustrative source deck, choose `MUnit=kg` rather than `mmol` in all three vapor decks. This avoids introducing a mass-unit conversion not represented by the pinned thermal input path. With cm/hour, dimensionful conversions from SI are:

```text
capacity [kg cm^-1 h^-2 K^-1]: SI / (100*(1/3600)^2) = SI * 129,600
conductivity [kg cm h^-3 K^-1]: SI * 100 / (1/3600)^3 = SI * 4.6656e12
dispersivity [cm]: SI metres * 100
```

Use manual-default phase capacities `Cn=1.92e6`, `Co=2.51e6`, `Cw=4.18e6` J m⁻³ K⁻¹, converted to `2.48832e11`, `3.25296e11`, `5.41728e11` in the project basis. These are coefficient conversions; the operative capacity is fraction-weighted by `TPar(1:2)` and `theta`.

| Texture | Solid `Qn` (θn) | Organic `Qo` (θo) | `Beta` λL (cm) | `B1` | `B2` | `B3` | `Cn` | `Co` | `Cw` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Sandy | 0.90 | 0.01 | 0.5 | 1.06368e12 | 1.12104e13 | 2.28924e13 | 2.48832e11 | 3.25296e11 | 5.41728e11 |
| Loamy | 0.60 | 0.02 | 0.5 | 1.1337408e12 | 1.8335808e12 | 7.1570304e12 | 2.48832e11 | 3.25296e11 | 5.41728e11 |
| Clayey | 0.50 | 0.03 | 0.5 | 1.06368e12 | 1.12104e13 | 2.28924e13 | 2.48832e11 | 3.25296e11 | 5.41728e11 |

Conductivity table arithmetic: multiply reported SI `b1..b3` by `4.6656e12`. Sandy uses published Chung-Horton sand coefficients `0.228,2.406,4.909`; loamy uses Example 7 values `0.243,0.393,1.534`; clayey uses the sand curve as an explicitly labeled fallback because the cited sources do not provide the clay triplet in searchable text. Thus the clay row is only a finite material-shaped stress input, not an authoritative clay curve. Fractions are representative choices with `0<Qn+Qo<1`, leaving the remainder as pore volume; they are not claimed as class averages. `Beta=0.5 cm` is an illustrative small dispersivity (not a documented universal textural-class default); it is selected for finite order-of-magnitude behavior and must be sensitivity-checked. The earlier heat-only deck's shorthand `0.5 0.2 0.0 ...` is not reused as a physical row.

These three rows are **approved illustrative fixture inputs** for order-of-magnitude and dimensional-analysis tests, not universal recommendations, measured values, or calibration. They require deck `MUnit=kg`, `iCampbell=0`, and finite physical porosity/material mapping. Before execution, verify selector/deck parsing and all boundary/input units, then document exact changed decks and hashes. Reuse the same selected row in the thermal and off-control pair so only `lVapor` differs; use the three distinct rows only if the cases support a material table without otherwise changing the comparison design.

## Scope and uncertainty

The manual/help and literature examples establish representative thermal-conductivity coefficients, while fractions and dispersivity remain explicit fixture assumptions. The source has a separate Campbell branch; these rows apply only to `iCampbell=0`. No physical-validation, calibration, or universal-material claim is made.
