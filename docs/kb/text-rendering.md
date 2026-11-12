# Text and images on surfaces

Last update: 2026-11-11

## Test method

| Fact | Confidence | Source |
|---|---|---|
| The test ran on an Apple M1 Max, arm64, with macOS 27.2 and Rust 1.99.1. It did not open a device. | Measured | T: system commands and test log |
| T is a test program, not in the repository. It uses `image` 0.25.10 with default features off and features `png` and `jpeg`. Each candidate has a separate release binary. The release profile has `strip = true`. | Measured | T: `Cargo.toml`, `Cargo.lock`, `bins/` |
| Each label has a new 112 × 112 `RgbImage`. The text area is 104 pixels wide. Each line is centred. The text block is centred in height. The test blends alpha into RGB. Core Text uses an RGBA bitmap and then copies RGB. | Measured | T: `src/main.rs` and candidate modules |
| The first time starts before font loading. It includes the canvas, layout, and drawing of `Volume up and down` at 18 pixels. It excludes process launch, PNG encoding, and file I/O. | Measured | T: `Instant` in `src/main.rs` |
| The later time is the mean of 100 calls with the same text and size. Each call makes a new canvas and layout. Font state stays. COSMIC Text keeps a `SwashCache`. Parley keeps a `ScaleContext`, but no glyph bitmap cache. The other candidates have no test-owned glyph bitmap cache. | Measured | T: candidate modules and `black_box` loop |
| The main results are medians from five new processes for each candidate. The order changes on each pass. OS font and file caches were not cleared. These are first-label times, not cold-boot or full application start times. | Measured | T: `measure.py`, `run-*.txt`, `measurements.json` |
| Memory is resident size (RSS), in KiB, from `ps -o rss= -p PID`. The test samples before font loading, after the first label, and after the five script samples. The last sample is before PNG encoding. | Measured | T: `rss_kib` and `rss_scripts_kib` |
| RSS includes shared resident pages. It is not physical footprint. It does not count a system font service in another process. The before sample already includes loaded frameworks. The change alone does not give the full cost of a library. | Source | macOS `ps(1)`; T: sample positions |
| The extra crate count is the set difference from the image-only baseline. The command is `cargo tree -e normal,build --prefix none --format '{p}'`, for the host target. A name and version pair counts once. The count includes the candidate and build dependencies. It excludes dev dependencies and inactive targets. | Measured | T: `tree-*.txt`, `measure.py` |
| The extra binary size is the file size minus the image-only binary. All binaries have the same profile and PNG writer. The baseline is 582 864 bytes. The single-font binaries include the font bytes. System frameworks and system font files are not in the binary size. | Measured | T: `bins/`, `measure.py` |

## Candidates and fonts

| Fact | Confidence | Source |
|---|---|---|
| COSMIC Text 0.19.0 uses its default features: `std`, `swash`, and `fontconfig`. The test uses system fonts, `Family::SansSerif`, advanced shaping, and `Wrap::WordOrGlyph`. Its line height is 1.3 times the size. | Measured | T: `src/cosmic.rs`, manifest; C |
| fontdue 0.9.5 uses its default features: `simd` and `hashbrown`. The test uses its word wrap and middle/centre alignment. | Measured | T: `src/due.rs`, manifest; F |
| ab_glyph 0.2.32 uses its default features: `std`, `variable-fonts`, and `gvar-alloc`. The test adds word wrap, kerning, and centre alignment. It converts pixels per em to the height scale that ab_glyph needs. | Measured | T: `src/ab.rs`, manifest; A |
| Both single-font tests embed the same `NotoSans[wdth,wght].ttf`, at the default axis values. The file has 2 049 096 bytes. Its SHA-256 is `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d`. | Measured | T: `NotoSans.ttf`, `wc -c`, `shasum -a 256`; N |
| The font licence is SIL Open Font License 1.1. It permits redistribution with the licence and its conditions. The test keeps `OFL.txt` with the font. | Source | N: `OFL.txt` |
| Core Text uses the Rust binding `core-text` 22.0.0, `core-graphics` 0.25.0, and `core-foundation` 0.10.1. All have default features. The test starts with Helvetica and lets Core Text select fallback fonts. It turns font smoothing off and keeps antialiasing on. | Measured | T: `src/ct.rs`, manifest; CT |
| Parley 0.12.0 built without a workaround. It uses its default `system` feature and its default sans-serif family. The test adds Swash 0.2.6 with default features to draw masks, outlines, and colour emoji. | Measured | T: `src/parley_test.rs`, release build log; P, S |
| Font families and line metrics differ between candidates. Thus the test compares usable label stacks, not rasterizers with identical glyph positions. | Inference | T: font choices, dimensions, and PNG files |

## Time and memory

All rows below are **Measured**. The source is T: `measurements.json`.
Each row also gives that label and source for use on its own.

| Candidate | First label, ms | Range of first times, ms | Mean of 100 later labels, ms | Confidence | Source |
|---|---:|---:|---:|---|---|
| Image only | 0.018 | 0.013–0.038 | 0.005 | Measured | T |
| COSMIC Text | 31.083 | 28.987–72.905 | 0.044 | Measured | T |
| fontdue | 14.929 | 14.821–21.945 | 0.021 | Measured | T |
| ab_glyph | 0.110 | 0.102–0.560 | 0.040 | Measured | T |
| Core Text | 8.743 | 7.706–10.172 | 0.046 | Measured | T |
| Parley with Swash | 52.744 | 52.563–67.374 | 0.162 | Measured | T |

| Candidate | RSS before, KiB | RSS after first, KiB | RSS change, KiB | RSS after scripts, KiB | Confidence | Source |
|---|---:|---:|---:|---:|---|---|
| Image only | 1 952 | 2 080 | 128 | 2 208 | Measured | T |
| COSMIC Text | 5 584 | 10 320 | 4 752 | 12 592 | Measured | T |
| fontdue | 1 952 | 21 536 | 19 584 | 21 920 | Measured | T |
| ab_glyph | 1 984 | 2 496 | 512 | 2 960 | Measured | T |
| Core Text | 5 808 | 11 472 | 5 648 | 19 584 | Measured | T |
| Parley with Swash | 5 968 | 18 048 | 12 080 | 19 968 | Measured | T |

| Fact | Confidence | Source |
|---|---|---|
| Each column above is a separate median. RSS values have page-size steps. A difference between two column medians need not equal the median of the changes. | Measured | T: five raw runs and summary calculation |
| An earlier exploratory process took 250.881 ms for the first COSMIC Text label, 19.498 ms for fontdue, 0.331 ms for ab_glyph, 12.677 ms for Core Text, and 119.585 ms for Parley. These times are not in the five-run median. | Measured | T: `first-run.txt` |
| First use can cost more than a repeated process test shows. The full application must still be checked against the product targets. This test does not prove those targets. | Inference | T: first-use and repeat results; [product targets](../ROADMAP.md#product-targets) |

## Crate and binary cost

| Candidate | Extra crates | Release binary, bytes | Extra bytes | Confidence | Source |
|---|---:|---:|---:|---|---|
| COSMIC Text | 37 | 2 663 536 | 2 080 672 | Measured | T: tree set difference, `bins/cosmic` |
| fontdue | 8 | 2 799 600 | 2 216 736 | Measured | T: tree set difference, `bins/due` |
| ab_glyph | 4 | 2 782 512 | 2 199 648 | Measured | T: tree set difference, `bins/ab` |
| Core Text | 13 | 622 032 | 39 168 | Measured | T: tree set difference, `bins/ct` |
| Parley with Swash | 60 | 2 748 368 | 2 165 504 | Measured | T: tree set difference, `bins/parley-test` |

## PNG checks

| Fact | Confidence | Source |
|---|---|---|
| Each candidate writes 30 PNG files. All have 112 × 112 pixels. The Latin cases are `Mail` and `Volume up and down`, at 10, 12, 14, 18, and 24 pixels. Each has white text on RGB (24, 32, 48), and black text on RGB (238, 234, 224). | Measured | T: `src/main.rs`, PNG files |
| The script cases are `Café Ünïcode`, `नमस्ते`, `音量`, `مرحبا`, and `🔊`, at 18 pixels, in both colour cases. Colour emoji keep their own colours. | Measured | T: `src/main.rs`, PNG files |
| At 10 pixels, the wrap string fits on one line with some candidates. It wraps at larger sizes. The test keeps the same width rather than forcing an extra line. | Measured | T: PNG files |
| The agent read the PNG files as images. The contact sheets have size columns 10, 12, 14, 18, 24. Rows 1–4 are Mail dark, Mail light, wrap dark, wrap light. Rows 5–6 are the five script cases, dark and light. The sheets use nearest-neighbour enlargement by two. | Measured | T: `src/bin/sheets.rs`, `*-contact-sheet-2x.png` |

| Candidate | Correct output | Boxes, missing output, or faults | Small sizes | Confidence | Source |
|---|---|---|---|---|---|
| COSMIC Text | Both Latin cases wrap and centre. Accents, Devanagari, CJK, connected right-to-left Arabic, and the colour emoji are visible. | No boxes. No blank sample. | 10 pixels is soft and small. 12 is clearer. 14 and above are easy to read in the PNG. | Measured | T: `cosmic-contact-sheet-2x.png` and original PNGs |
| fontdue | Latin, accents, and centred word wrap are visible. | CJK, Arabic, and emoji show boxes. Devanagari has glyphs, but its conjunct and mark placement are not correct. No blank sample. | 10 and 12 pixels are readable, but spacing has more whole-pixel steps. 14 and above are clearer. | Measured | T: `fontdue-contact-sheet-2x.png` and original PNGs |
| ab_glyph | Latin, accents, kerning, and test-owned word wrap are visible. | CJK, Arabic, and emoji show boxes. Devanagari has separate glyphs, not correct shaping. No blank sample. | 10 pixels is thin and soft. 12 is clearer. 14 and above are easy to read. | Measured | T: `ab_glyph-contact-sheet-2x.png` and original PNGs |
| Core Text | Latin, accents, centred word wrap, Devanagari, CJK, connected right-to-left Arabic, and the colour emoji are visible. | No boxes. No blank sample. | 10 pixels is soft but readable. 12 is clearer. 14 and above are easy to read. Both colour cases have clean edges. | Measured | T: `core-text-contact-sheet-2x.png` and original PNGs |
| Parley with Swash | Latin, accents, centred word wrap, Devanagari, CJK, connected right-to-left Arabic, and the colour emoji are visible. | No boxes. No blank sample. | 10 pixels is small and soft. 12 is clearer. 14 and above are easy to read. | Measured | T: `parley-contact-sheet-2x.png` and original PNGs |

| Fact | Confidence | Source |
|---|---|---|
| These checks cover only the named strings and installed fonts. They are not a review by readers of each language. They do not prove correct output for all scripts or emoji sequences. | Inference | T: test coverage |
| The file pattern is `<library>-112x112-<size>px-<colour>-<case>.png`. The library names are `cosmic`, `fontdue`, `ab_glyph`, `core-text`, and `parley`. The colour names are `white-dark` and `black-light`. | Measured | T: PNG files |

## Engine operations

The following rows describe published APIs. Only measurement, word wrap, and
centred drawing were exercised in T. Shrink and ellipsis are work for the
label module, not results of this test.

| Candidate | Width and height | Wrap | Make smaller or cut with `…` | Confidence | Source |
|---|---|---|---|---|---|
| COSMIC Text | Shape a buffer. Read `layout_runs()`: maximum `line_w`, and last `line_top + line_height`. | `set_size` and `Wrap::WordOrGlyph`. | Set smaller `Metrics`, then shape again. `set_ellipsize(Ellipsize::End(EllipsizeHeightLimit::Height(h)))` supplies a height limit. | Source | C: `buffer.rs`, `layout.rs` |
| fontdue | `Layout::height()`, line padding relative to `max_width`, and glyph bounds. | `LayoutSettings::max_width` and `WrapStyle::Word`. | Layout again with a smaller `TextStyle` size. To cut, remove complete text clusters, append `…`, and measure again. The engine must supply this logic and shaping/fallback if needed. | Source | F: `src/layout.rs`, README |
| ab_glyph | Sum scaled advances and kerning. Read ascent, descent, line gap, and outline pixel bounds. | No paragraph wrapper. The test adds a word loop. General text needs line-break and shaping logic. | Change scale and measure again. The engine must select complete clusters, append `…`, and measure again. It must also supply fallback, bidirectional layout, and shaping. | Source | A: `Font`, `ScaleFont`, `OutlinedGlyph`; test word loop in T |
| Core Text | `CTLine::get_typographic_bounds()` gives width, ascent, descent, and leading. `CTFramesetter::suggest_frame_size_with_constraints()` gives a constrained size and fit range. T also measures the first and last line origins. | `CTFramesetter::create_frame()` in a path of the required width. Its lines have fallback fonts and shaping. | Clone the font at a smaller size and make the frame again. For a cut, use `CTLineCreateTruncatedLine` with an ellipsis token, or remove complete clusters and make the frame again. The binding does not wrap that truncation function. A small FFI wrapper is necessary for it. Fit ranges use UTF-16 units, not UTF-8 byte offsets. | Source | CT: `font.rs`, `line.rs`, `frame.rs`, `framesetter.rs`; Apple Core Text headers |
| Parley | `Layout::width()` and `height()` after line breaking. Runs have glyph positions and font metrics. | `break_all_lines(Some(width))`, then `align`. | Build again at a smaller `FontSize`. The engine must select a text-cluster boundary, add `…`, and build again for a cut. It also needs a rasterizer, such as Swash. | Source | P: layout and run APIs; S |

| Fact | Confidence | Source |
|---|---|---|
| A label implementation can try sizes from large to small until the measured width and height fit. It can then cut the last visible line if the minimum size does not fit. It must not split a UTF-8 code point or a text cluster. | Inference | The APIs above |
| `band_height` can use the same layout and line metrics as `draw`. It must cap the result to the surface height. Keeping the two operations on one layout path avoids different wrap results. | Inference | The APIs above; label interface in the task brief for issue #50 |

## Licences and platforms

| Candidate | Library licence | Linux and Windows | Confidence | Source |
|---|---|---|---|---|
| COSMIC Text | MIT OR Apache-2.0 | Both supported, with system-font discovery and fallback. Not run here. | Source | C: manifest and README |
| fontdue | MIT OR Apache-2.0 OR Zlib | Portable Rust, with an embedded font. No platform font service is needed. Not run here. | Source | F: manifest and README |
| ab_glyph | Apache-2.0 | Portable Rust, with an embedded font. No platform font service is needed. Not run here. | Source | A: manifest and font APIs |
| Core Text binding and the two helper bindings | MIT OR Apache-2.0 | No. Core Text is an Apple framework. Linux and Windows need a different implementation. The test ships no Apple font file. | Source | CT: manifests, framework links, font-loading API |
| Parley with Swash | MIT OR Apache-2.0 for both | Both supported. Fontique has Windows and Linux system backends. Not run here. | Source | P: Fontique backend selection; S: manifest |

## Images and SVG cost

| Fact | Confidence | Source |
|---|---|---|
| In Phase 2, an icon is an image file of the user: PNG or JPEG. The `image` crate decodes it. A text library is not necessary for that operation. | Source | Issue #50 task brief; `image` 0.25.10 decoder APIs; [Phase 2](../ROADMAP.md#phase-2-first-gui-see-and-paint-the-device-complete) |
| The resize test uses `image` 0.25.10, a release build, and the mean of 100 calls. The source is synthetic RGB. No decode is in the time. | Measured | T: earlier resize test, `src/resize.rs`; supplied measurement record from issue #50 work |
| `resvg` 0.49.0 with default features adds 47 crates above this image-only baseline, on this host. Its default features are `svgz`, `text`, `system-fonts`, `memmap-fonts`, and `raster-images`. | Measured | T: `tree-svg.txt`, set difference; R: manifest |
| The `resvg` licence is MIT OR Apache-2.0. | Source | R: manifest |
| This is a dependency-cost check only. No SVG output, time, memory, or binary cost was tested. | Measured | T: test scope |

| Source size | Destination size | Lanczos3, ms | CatmullRom, ms | Triangle, ms | Confidence | Source |
|---|---|---:|---:|---:|---|---|
| 512 × 512 | 112 × 112 | 2.637 | 1.682 | 0.901 | Measured | Earlier resize test in T |
| 1024 × 1024 | 112 × 112 | 9.175 | 5.872 | 3.093 | Measured | Earlier resize test in T |

## The label module of the engine

The module `crates/engine/src/label.rs` uses Core Text. The tests of the
engine make these images on macOS, on 2026-11-11.

| Fact | Confidence | Source |
|---|---|---|
| The module uses the system font in the weight for emphasis (`kCTFontEmphasizedSystemFontType`) for a bold label, and `kCTFontSystemFontType` for a regular label (issue #189, 2026-11-13). It tries the size of the label (20 pixels by default), and then 2 pixels smaller each time to 10 pixels. It uses the first size at which the text fits. | Measured | The code and its tests |
| A regular label of "Mail" at 20 pixels has less ink than a bold label: the bold label has more than 110 % of its pixels. A label at 32 pixels is more than 10 pixels wider than at 20 pixels. | Measured | Test `the_size_and_the_weight_change_the_text`, 2026-11-13 |
| The module puts the text block at the top, at the centre, or at the bottom of its area. On a key face with no image, a label at the top starts less than 14 pixels from the top edge, and a label at the bottom ends less than 14 pixels from the bottom edge. | Measured | Tests in `crates/engine/src/render.rs` and `crates/engine/tests/engine/labels.rs`, 2026-11-13 |
| A text that does not fit at 10 pixels is cut, and gets `…` at the end. The label stays in its area. | Measured | Test with a text of 880 characters |
| One word of 60 letters with no space goes to more lines and stays in the area | Measured | Test |
| Devanagari, CJK, Arabic, and an emoji each give a label on a key face | Measured | Test. An agent looked at one image with Devanagari, CJK, and an emoji. |
| A label of 1 line with an image has a band of 16 to 30 pixels on a key face. The band is at most 40 % of the height of the surface. | Measured | Test |
| An agent looked at 8 key faces and 8 bar panels, with and without an image of the user. Each label was at the centre of its area, and no label was outside its surface. | Measured | A test program, not in the repository |

## Not known

- The physical footprint of the full application with GPUI and the label
  implementation. RSS from this test cannot answer that question.
- The full application cold start with an empty OS font cache.
- Label quality on the device. This test used no device.
- Mixed-direction text, other scripts, long unbroken words, and emoji sequences.
- Whether a cut label can show an incorrect sign. The label module cuts
  between two characters, and not between two groups of characters that make
  one sign.
- The effect of other installed fonts and another macOS version.
- Performance and output on Linux and Windows. Core Text needs a replacement
  there. The other candidates were tested only on macOS.
- A full licence audit of transitive crates. The table gives candidate licences.
- SVG time, memory, binary cost, and product need. SVG has no decision here.

## Sources

- **T:** A test program, not in the repository. The retained files include
  `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/cosmic.rs`, `src/due.rs`,
  `src/ab.rs`, `src/ct.rs`, `src/parley_test.rs`, `src/resize.rs`,
  `src/bin/core_text_example.rs`, `src/bin/sheets.rs`, `measure.py`,
  `measurements.json`, `first-run.txt`, `run-*.txt`, `tree-*.txt`, and PNG files.
  The source and output stay outside the repository. No personal path is
  recorded here.
- **C:** COSMIC Text 0.19.0, published source and manifest:
  <https://github.com/pop-os/cosmic-text>.
- **F:** fontdue 0.9.5, published source and manifest:
  <https://github.com/mooman219/fontdue>.
- **A:** ab_glyph 0.2.32, published source and manifest:
  <https://github.com/alexheretic/ab-glyph>.
- **CT:** `core-text` 22.0.0, `core-graphics` 0.25.0, and `core-foundation`
  0.10.1, published source and manifests:
  <https://github.com/servo/core-foundation-rs>. Core Text framework headers
  in the installed macOS SDK give `CTLineCreateTruncatedLine`.
- **P:** Parley and Fontique 0.12.0, published source and manifests:
  <https://github.com/linebender/parley>.
- **S:** Swash 0.2.6, published source and manifest:
  <https://github.com/dfrg/swash>.
- **N:** Google Fonts, Noto Sans font and `OFL.txt`:
  <https://github.com/google/fonts/tree/main/ofl/notosans>.
  The last change to that directory when fetched was commit
  `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5`.
- **R:** resvg 0.49.0, published manifest:
  <https://github.com/linebender/resvg>.
