# KONCET Design System

> **Status:** Baseline · **Applies to:** `v0.1.0`  
> **Last updated:** 2026-11-24  
> **Related:** [Screens and flows](screens-and-flows.md) · [PRD principles](../product/prd.md#4-product-principles)

## 1. Brand direction

**Personality:** precise, calm, capable, transparent. KONCET should feel like a well-made
instrument for observing a system — not a game console, a cloud marketing dashboard, or a
cyberpunk terminal.

**Design pillars**

1. **Simulation is always visible.** A restrained “SIMULATED” marker and honest copy, never
   fake real-world indicators.
2. **State over decoration.** Status, time, and cause are the visual hierarchy.
3. **Calm density.** High information density with generous grouping, clear rules, and
   consistent alignment.
4. **One accent, disciplined status colours.** The interface is mostly neutral; colour means
   something.
5. **Native desktop behaviour.** Keyboard, focus, resizing, and window state are first-class.

**Avoid:** neon gradients, glassmorphism, dark-pattern motion, oversized hero art inside the
workbench, mixed icon styles, more than one brand accent, colour-only status, and decorative
animation in logs or charts.

## 2. Identity

- Wordmark: `KONCET` in Inter SemiBold, uppercase, +0.08em tracking. No gradient.
- Product line: “Deployment simulation workbench”.
- Monogram direction (future asset): a `K` formed from stacked horizontal layers/brackets,
  usable as a 16 px favicon and AppImage icon. Monochrome first; one-colour inverted variant.
- Tone of voice: direct, technically accurate, non-hyped. Say “simulated container”, not
  “container”; say “virtual port”, not “port”.

## 3. Colour

### 3.1 Material 3 scheme

Use Material 3 `ColorScheme.fromSeed` with a **fixed** light and dark scheme. Dynamic colour
(wallpaper extraction) is disabled for consistency.

- Canonical seed: **KONCET Teal `#00696E`**
- Light surface family: `surface` `#F4FAF9`, `surfaceContainerLowest` `#FFFFFF`,
  `surfaceContainerLow` `#EEF5F3`, `surfaceContainer` `#E8EFED`,
  `surfaceContainerHigh` `#E2E9E7`
- Dark surface family: `surface` `#0D1514`, `surfaceContainerLowest` `#08100F`,
  `surfaceContainerLow` `#151D1C`, `surfaceContainer` `#192120`,
  `surfaceContainerHigh` `#242C2B`

The theme is generated during implementation with Material Theme Builder and checked in as
Dart tokens. The seed and surface anchors above are authoritative; components must use
`ColorScheme` roles, never raw palette values.

### 3.2 Semantic status extension

Status colours are a `ThemeExtension<KoncetStatusColors>`, not part of the seed scheme.

| State | Light | Dark | Used for |
| --- | --- | --- | --- |
| Healthy / running | `#1B7F4B` | `#6FD79A` | healthy deployment, succeeded, running |
| Simulated / neutral active | `#00696E` | `#80D5D0` | simulation markers, paused clock accent |
| Warning / degraded | `#8A5A00` | `#F2C14E` | degraded, retrying, warning |
| Danger / failed | `#B3261E` | `#FFB4AB` | failed, error logs, destructive actions |
| Stopped / neutral | `#5C6462` | `#B8C4C1` | stopped, disabled, cancelled |
| Info | `#0061A4` | `#9ECAFF` | neutral information, build running |

Rules:

- Every status uses **icon + text label**, not colour alone.
- Status containers use low-emphasis tinted surfaces; saturated colour is reserved for icons,
  badges, and chart strokes.
- Error red is never used for decorative emphasis.
- Contrast for text on tinted status surfaces must meet WCAG AA (4.5:1 for body text).

### 3.3 Data visualisation palette

Use role-based series colours in this order: primary, tertiary, info, warning, healthy, muted.
Strokes 2 px; area fills ≤ 10% opacity; grid lines `outlineVariant`; axis labels
`onSurfaceVariant`; no rainbow palettes. Series must also be distinguishable by line style or
direct labels, not only hue.

## 4. Typography

Bundle **Inter** (UI) and **JetBrains Mono** (code, logs, identifiers, clock). Both are SIL OFL
and must ship with licence texts. Do not fetch fonts at runtime.

| Token | Face / weight | Size / line height | Usage |
| --- | --- | --- | --- |
| `displaySmall` | Inter SemiBold 600 | 32 / 40 | Welcome/empty-state headline only |
| `headlineSmall` | Inter SemiBold 600 | 24 / 32 | Page titles |
| `titleLarge` | Inter SemiBold 600 | 20 / 28 | Section/dialog titles |
| `titleMedium` | Inter Medium 500 | 16 / 24 | Cards, list group headers |
| `titleSmall` | Inter Medium 500 | 14 / 20 | Table headers, labels |
| `bodyLarge` | Inter Regular 400 | 16 / 24 | Long-form onboarding |
| `bodyMedium` | Inter Regular 400 | 14 / 20 | Default body and controls |
| `bodySmall` | Inter Regular 400 | 12 / 16 | Secondary metadata |
| `labelLarge` | Inter Medium 500 | 14 / 20 | Buttons, tabs |
| `labelMedium` | Inter Medium 500 | 12 / 16 | Chips, badges |
| `monoSmall` | JetBrains Mono Regular 400 | 12 / 18 | Logs, code, IDs, seeds |
| `monoMedium` | JetBrains Mono Medium 500 | 13 / 20 | Sim clock, metric values |

- Use tabular numerals for clocks, metrics, tables, and logs (`fontFeatures: [tnum]`).
- Do not use all-caps for body text; only for wordmark and short overlines with tracking.
- Minimum body size 12 px; minimum log size 12 px. Respect OS text scaling up to 200%.
- Truncate with tooltip/expand affordance; never clip identifiers silently.

## 5. Spacing, shape, and density

**Spacing scale (4 px base):** 4, 8, 12, 16, 20, 24, 32, 40, 48, 64.  
**Layout:** page padding 24 px (32 px at ≥1440); gutters 16/24; section gap 24; control gap 8/12.

| Token | Value | Usage |
| --- | --- | --- |
| `radius.xs` | 4 px | Checkboxes, small tags |
| `radius.sm` | 8 px | Inputs, menu items |
| `radius.md` | 12 px | Cards, panels, popovers |
| `radius.lg` | 16 px | Dialogs, large containers |
| `radius.full` | 999 px | Status chips, avatar-like marks |

- Material 3 desktop density: **compact** for tables and toolbars; standard for forms and
  dialogs. Default control height 36 px; table row 36 px; top bar 56 px; navigation rail
  232 px expanded / 80 px collapsed.
- Surfaces are mostly tonal. Shadows only for dialogs, menus, and floating overlays; use
  `surfaceContainer*` roles instead of stacking elevations.
- Dividers: 1 px `outlineVariant`. Cards use a hairline border and no heavy shadow.
- Content maximum width 1440 px; readable prose max 72 characters.

## 6. Iconography

- Material Symbols Rounded, bundled, 20 px in controls and 24 px in navigation/headers.
- Default: outlined; filled only for selected navigation or strong status.
- Icons always pair with labels in navigation, status, and destructive actions.
- Custom illustrations are limited to empty states; line-based, one accent, no stock 3D art.
- Never use emoji as interface iconography.

## 7. Motion

| Token | Duration | Usage |
| --- | --- | --- |
| `motion.micro` | 100 ms | Hover, focus, small state change |
| `motion.short` | 150 ms | Menus, chips, tab indicator |
| `motion.medium` | 200 ms | Panels, expanding sections |
| `motion.long` | 300 ms | Dialogs, route transitions |

Use standard Material easing; no springy overshoot in operational surfaces. Respect
`MediaQuery.disableAnimations`. Live logs and charts update without entrance animations.
Speed changes of the simulation clock are discrete and labelled, not animated numbers.

## 8. Layout system

- Desktop-first, single window. Minimum 1024×720; reference 1440×900.
- App shell: 56 px top bar + navigation rail + content.
- Rail is 232 px when expanded (≥1280 px), 80 px icon-only when 1024–1279 px.
- Project content uses a 12-column mental grid with 24 px gutters; inspector panels are 320 px.
- Run workspace uses a persistent session header (run identity, clock, controls) and tabs below.
- Tables: sticky header, 36 px rows, horizontal scroll with frozen first column, right-aligned
  numerics, no zebra stripes unless needed for long rows.
- Logs: monospace, line height 18, optional timestamp/source columns, wrap off by default.
- Empty states appear inside the content region, not as full-screen blocking modals.

## 9. Component inventory

All shared components live in `packages/koncet_ui`. Apps compose them; no private one-off
variants in screens.

| Component | Responsibility |
| --- | --- |
| `KoncetAppShell` | Top bar, rail, content, active-run indicator |
| `KoncetPageHeader` | Title, breadcrumb, description, primary actions |
| `KoncetButton` | Primary/secondary/tertiary/danger buttons with loading/disabled states |
| `KoncetStatusBadge` | Icon + label + tone for one canonical status |
| `KoncetSimulatedTag` | Consistent “SIMULATED” marker |
| `KoncetSectionCard` | Titled surface group with optional action |
| `KoncetEmptyState` | Icon, explanation, primary action, optional hint |
| `KoncetErrorState` | Error summary, code, retry/copy diagnostics |
| `KoncetDataTable` | Sticky header, empty/loading, row selection, keyboard support |
| `KoncetCodeEditor` | File editing surface with line numbers, find, language hint |
| `KoncetFileTree` | Virtual source tree with create/rename/delete affordances |
| `KoncetSimClock` | Monospace elapsed time, speed, pause state |
| `KoncetRunControls` | Start/pause/resume/stop and speed menu |
| `KoncetPipeline` | Four-stage build progress with statuses and durations |
| `KoncetTimeline` | Ordered events, icons, severity, link targets |
| `KoncetLogViewer` | Virtualised log list, filters, follow, wrap, copy |
| `KoncetMetricTile` | Label, current value, unit, sparkline |
| `KoncetChart` | Accessible line/area chart with labels and empty state |
| `KoncetReasonCallout` | Primary reason plus actionable explanation |
| `KoncetConfirmDialog` | Destructive confirmation with consequence copy |
| `KoncetValidationMessage` | Field/path-linked inline error |

Status→visual mapping is centralised once. Components accept a semantic status enum, not
arbitrary colours.

`packages/koncet_ui` contains the theme, status colours, typography extension, and the
presentational components in the table. Data-bound widgets — the source editor with autosave,
log viewer, metric charts, and run-specific panels — live in `apps/desktop/lib/src/widgets` and
compose those primitives. The design package never imports the bridge, app state, or SQL, so it
can be tested without a database.

## 10. Core interaction patterns

- **Primary action** is one filled button per surface (Start run; Create project; Save).
- **Destructive actions** use the danger role, an explicit consequence sentence, and a
  confirm action labelled with the verb (“Delete project”), never “OK”.
- **Autosave** for source/scenario edits: debounce, show `Saving…` / `Saved`, never block
  navigation without warning if a write failed.
- **Long operations** show determinate progress when duration is known (build stages) and
  busy indicators otherwise, always with a cancel/stop path where safe.
- **Errors** are contextual first (field, stage, route), global only for app-level failures.
- **Notifications** use inline status/snackbar for transient success; no toast for critical
  failure. Errors stay visible until resolved or dismissed.
- **Tooltips** explain icons and truncated values; never the only source of essential info.
- **Focus** is always visible and follows logical order. Escape closes overlays.
- **Loading** uses skeletons for data views, not spinners on a blank page.

## 11. Keyboard and accessibility

- Minimum target 32×32 px for pointer targets; 24 px only for secondary inline icons.
- WCAG 2.1 AA: 4.5:1 body text, 3:1 large text/UI boundaries, visible focus.
- Every icon-only button has a semantic label and tooltip.
- Logs and timeline are navigable and copyable; screen readers announce filter results and new
  errors (not every new log line).
- Charts provide a summary, accessible data table alternative, and non-colour encoding.
- Reduced motion disables transitions; no essential information is conveyed by animation.
- Text scaling up to 200% must not clip actions or hide status.
- Shortcuts in `v0.1`: `Ctrl+S` save, `Ctrl+Enter` start/run, `Esc` close overlay. A full
  command palette is deferred.

## 12. Consistency rules (review checklist)

- [ ] Uses design tokens from `koncet_ui`; no hard-coded colours, radii, or spacing in screens.
- [ ] Status always has icon + label and comes from the canonical mapping.
- [ ] “Simulated” is visible in every run context and networking surface.
- [ ] Empty, loading, error, retry, and disabled states exist.
- [ ] Destructive flows explain consequences and use the verb on the action.
- [ ] Keyboard path works without a mouse; focus order is logical.
- [ ] Light and dark screenshots reviewed at 1440×900 and 1024×720.
- [ ] Text at 200% and display scaling at 150% remain usable.
- [ ] Numbers use tabular figures; logs use mono; headings follow the type scale.
- [ ] No new icon style, chart palette, or component variant without updating this document.
