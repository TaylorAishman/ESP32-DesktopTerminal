# Icons

Each `*.svg` here becomes an `Icon` variant in `dt-ui` (file `rain_heavy.svg`
becomes `Icon::RainHeavy`). The `dt-ui` build script rasterises every icon at
each size in `SIZES` (`crates/dt-ui/build.rs`) and thresholds it to 1 bit:
pixels at least half covered become black.

Guidelines, so icons stay legible at 16 px on e-ink:

- Use `viewBox="0 0 48 48"`, black (`#000`) on transparent.
- Strokes at least 3 units wide; avoid detail finer than 2 units.
- Keep a 2-unit margin inside the view box.
- Prefer filled shapes for small details.
