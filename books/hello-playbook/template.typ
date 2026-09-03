// Typography for `bower publish --target pdf`.
//
// A Typst template is a preamble: `#set` rules apply to everything after them,
// so this file is prepended to pandoc's output rather than wrapping it.
//
// Every font named here is checked against `typst fonts` before compiling. A
// silently substituted font is how a missing-glyph bug survives to a reader —
// see EPIC-06's corrigendum item 7.

#set page(paper: "us-letter", margin: 2.2cm, numbering: "1")
#set text(font: "Libertinus Serif", size: 10.5pt)
#set par(justify: true, leading: 0.62em)

// Code is the point of these books, so it gets the deliberate treatment: a
// monospace face with a full glyph repertoire, and a tinted block so a reader's
// eye finds it on the page.
#show raw: set text(font: "DejaVu Sans Mono", size: 8.5pt)
#show raw.where(block: true): it => block(
  fill: luma(247),
  inset: 8pt,
  radius: 3pt,
  width: 100%,
  it,
)

#show heading.where(level: 1): set text(size: 17pt)
#show link: set text(fill: rgb("#1a4a7a"))
