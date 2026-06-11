# PresetCSS

A utility-first CSS framework with a **radically simpler** configuration system.
One flat preset file in, one CSS file out compiled by a single Rust binary.
No JavaScript config, no `extend` merge magic, no plugin system for the 90% case.

```toml
# preset.toml - this is the whole config
[colors]
primary = "#3b82f6"

[breakpoints]
tablet = "768px"
```

```css
/* generated */
.bg-primary { background-color: #3b82f6; }
.text-primary { color: #3b82f6; }
.border-primary { border-color: #3b82f6; }
@media (min-width:768px) { .tablet\:flex { display: flex; } }
```

## Install / build

```sh
cargo build --release          # binary at target/release/presetcss
cargo install --path crates/presetcss-cli
```

## Commands

| Command | What it does |
|---|---|
| `presetcss init` | Write a commented `preset.toml` (a copy of the defaults). |
| `presetcss build [--minify] [--verbose] [-c file] [-o out.css]` | Generate CSS. |
| `presetcss watch` | Build, then rebuild on changes to the preset or content files. |
| `presetcss validate` | Check the preset for typos, bad hex, key collisions - no output written. |
| `presetcss explain <class>` | Show the CSS and preset provenance for one class. |
| `presetcss export --format json\|js\|css-vars` | Export design tokens. |
| `presetcss help theme` / `help static` | List token keys / built-in static utilities. |

## How config works (the only rule you must learn)

- A token group like `[colors]` is a **flat** `key = "value"` map. No nesting.
- Writing `[colors]` **replaces** the default color group.
- `[extend.colors]` **adds** to it (your keys win). That is the *only* merge.
- Variants are **opt-in per group** under `[variants]` - nothing is implicit.
- Need a one-off rule? `[utilities]` takes raw CSS. Still no JavaScript.

```toml
[extend.colors]      # keep defaults, add one color
brand = "#ff0050"

[variants]           # only colors get hover/focus; statics get breakpoints
colors = ["hover", "focus"]
static = ["responsive"]

[utilities]
btn = "padding: 0.5rem 1rem; border-radius: 6px; background: #3b82f6; color: white;"
```

## Variant syntax

`breakpoint:state:utility` - e.g. `tablet:hover:bg-primary`. Breakpoint names are
**yours** (from `[breakpoints]`), not a fixed `sm/md/lg`.

## Tree-shaking

Set `content = ["src/**/*.html"]` in `[options]` and `build` emits only the
classes it finds in those files. Near-misses are reported instead of silently
dropped: `'bg-primry' looks like a PresetCSS class but is not generated - did you mean 'bg-primary'?`

## Project layout

```
crates/presetcss-core/   # pure library: parse -> resolve -> [scan] -> generate -> emit
crates/presetcss-cli/    # the `presetcss` binary
examples/                # minimal + advanced presets
DESIGN.md                # full design document
```

## Develop

```sh
cargo test          # core has a snapshot-style unit suite
cargo clippy --all-targets
```

## License

GPL-3.0 © Seyyed Ali Mohammadiyeh (MAX BASE)
