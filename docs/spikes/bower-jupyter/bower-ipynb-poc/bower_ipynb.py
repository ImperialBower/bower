#!/usr/bin/env python3
"""bower_ipynb — proof of concept for `bower publish --target ipynb` (spec § 15).

Python stand-in for the bower-core kernel path, so the notebook target can be
tried end-to-end before the Rust renderer exists. Same contract as the spec:

  book markdown  ─parse─▶  blocks  ─group─▶  steps  ─fold─▶  tree states
                                       │                         │
                                  play cells ──attach──▶  nearest preceding step
                                       │                         │
                                       ▼                         ▼
                              one .ipynb per chapter    line-anchored source links

Usage:
  bower_ipynb.py build  book/src/ch01-ranks.md -o notebooks/ch01-ranks.ipynb
  bower_ipynb.py verify notebooks/ch01-ranks.ipynb
"""
from __future__ import annotations

import argparse
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

import nbformat
from nbformat.v4 import new_code_cell, new_markdown_cell, new_notebook

DIRECTIVE = re.compile(r"<!--\s*(?:bower|bf)\s+(.*?)\s*-->", re.S)
KV = re.compile(r'(\w+)\s*=\s*"([^"]*)"')
FENCE = re.compile(r"```(\w*)\n(.*?)\n```", re.S)
TREE_KEYS = {"file", "op", "region", "src", "paths"}


class PlanError(Exception):
    """Plan-time errors are first-class (§ 6.1)."""


@dataclass
class Block:
    keys: dict
    lang: str
    body: str
    line: int  # source line of the directive, for error messages


@dataclass
class Step:
    id: str
    repo: str
    blocks: list[Block] = field(default_factory=list)


# ---------------------------------------------------------------- parsing

def parse(text: str) -> list[tuple[str, object]]:
    """Split a chapter into ('prose', str) and ('block', Block) items."""
    items, pos, auto = [], 0, 0
    for d in DIRECTIVE.finditer(text):
        fence = FENCE.match(text, d.end() + 1) or FENCE.match(text, d.end() + 2)
        if not fence:
            raise PlanError(f"line {text.count(chr(10), 0, d.start()) + 1}: directive without a fenced block")
        prose = text[pos:d.start()].strip()
        if prose:
            items.append(("prose", prose))
        keys = dict(KV.findall(d.group(1)))
        if "repo" not in keys:
            raise PlanError(f"line {text.count(chr(10), 0, d.start()) + 1}: directive missing repo=")
        if "step" not in keys and keys.get("notebook") != "play":
            auto += 1
            keys["step"] = f"auto-{auto:03d}"
        items.append(("block", Block(keys, fence.group(1), fence.group(2),
                                     text.count("\n", 0, d.start()) + 1)))
        pos = fence.end()
    tail = text[pos:].strip()
    if tail:
        items.append(("prose", tail))
    return items


# ---------------------------------------------------------------- tree fold (pure)

def strip_hidden(body: str) -> str:
    """mdBook `# ` hidden lines are part of the file; the prefix is not."""
    out = []
    for ln in body.split("\n"):
        out.append(ln[2:] if ln.startswith("# ") else ("" if ln == "#" else ln))
    return "\n".join(out)


def apply(tree: dict[str, str], b: Block) -> None:
    op, path, body = b.keys.get("op", "create"), b.keys["file"], strip_hidden(b.body)
    if op == "create":
        if path in tree:
            raise PlanError(f"line {b.line}: create of existing file {path}")
        tree[path] = body
    elif op == "replace":
        if path not in tree:
            raise PlanError(f"line {b.line}: replace of file never created: {path}")
        tree[path] = body
    elif op == "append":
        tree[path] = tree.get(path, "") + "\n" + body
    elif op == "region":
        name = b.keys["region"]
        begin, end = f"// bower:begin {name}", f"// bower:end {name}"
        cur = tree.get(path, "")
        if begin not in cur:
            raise PlanError(f"line {b.line}: region '{name}' before its markers exist in {path}")
        head, rest = cur.split(begin, 1)
        _, tail = rest.split(end, 1)
        tree[path] = f"{head}{begin}\n{body}\n{end}{tail}"
    else:
        raise PlanError(f"line {b.line}: op '{op}' not implemented in POC")


def fold(steps: list[Step]) -> dict[str, dict[str, str]]:
    """Step id -> full tree after that step. Pure function of the chapter."""
    tree, states = {}, {}
    for s in steps:
        for b in s.blocks:
            apply(tree, b)
        states[s.id] = dict(tree)
    return states


# ---------------------------------------------------------------- display (§ 3.4)

def shown_lines(body: str) -> list[str]:
    """Visible lines: drop `# ` hidden lines; honour bower:show markers."""
    lines = [ln for ln in body.split("\n") if not (ln.startswith("# ") or ln == "#")]
    if not any("bower:show" in ln for ln in lines):
        return [ln for ln in lines if not ln.strip().startswith("// bower:")]
    out, on = [], False
    for ln in lines:
        s = ln.strip()
        if s.startswith("// bower:show end"):
            on = False
        elif s.startswith("// bower:show"):
            on = True
        elif on:
            out.append(ln)
    return out


def line_range(tree_file: str, shown: list[str]) -> tuple[int, int]:
    """Where the displayed span lives in the folded file (1-based, inclusive).

    A stale anchor is a build error, not a reader's discovery."""
    hay = [(n + 1, ln.strip()) for n, ln in enumerate(tree_file.split("\n")) if ln.strip()]
    needle = [ln.strip() for ln in shown if ln.strip()]
    for i in range(len(hay) - len(needle) + 1):
        if [h for _, h in hay[i:i + len(needle)]] == needle:
            return hay[i][0], hay[i + len(needle) - 1][0]
    raise PlanError("displayed span not found in folded tree — line map broken")


# ---------------------------------------------------------------- render

def build(md: Path, cfg: dict, out: Path) -> None:
    items = parse(md.read_text())
    steps: dict[str, Step] = {}
    order: list[Step] = []
    # group blocks into steps (document order, § 4)
    for kind, it in items:
        if kind == "block" and it.keys.get("notebook") != "play":
            sid = it.keys["step"]
            if sid not in steps:
                steps[sid] = Step(sid, it.keys["repo"])
                order.append(steps[sid])
            steps[sid].blocks.append(it)
    states = fold(order)
    seq = {s.id: i + 1 for i, s in enumerate(order)}

    nb = new_notebook()
    nb.metadata["bower"] = {"book": cfg["book"]["name"], "chapter": md.stem}
    nb.cells.append(new_markdown_cell(
        f"> **Playable chapter** — generated by Bower from `{md.name}`; do not edit, edit the book. "
        "Rust cells are read-only views of the generated repo; Python cells are live."))

    last_step: Step | None = None
    play_n = 0
    for kind, it in items:
        if kind == "prose":
            nb.cells.append(new_markdown_cell(it))
            continue
        b: Block = it
        repo = cfg["repos"][b.keys["repo"]]
        if b.keys.get("notebook") == "play":
            # § 15.1 — play cells: attach to nearest preceding step, refuse tree keys
            bad = TREE_KEYS & b.keys.keys()
            if bad:
                raise PlanError(f"line {b.line}: play cell carries tree-affecting keys {sorted(bad)}")
            if "step" in b.keys:
                target = steps.get(b.keys["step"])
                if target is None:
                    raise PlanError(f"line {b.line}: play cell names unknown step '{b.keys['step']}'")
            else:
                target = last_step
                if target is None:
                    raise PlanError(f"line {b.line}: play cell has no step to attach to")
            if play_n == 0:
                nb.cells.append(new_code_cell(
                    f"%pip install -q \"{repo['notebook']['wheel']}\"  # pinned to {md.stem}-end",
                    metadata={"bower": {"kind": "pin"}, "tags": ["bower-pin"]}))
            play_n += 1
            meta = {"kind": "play", "step": target.id, "expect": b.keys.get("expect", "pass")}
            cell = new_code_cell(b.body, metadata={"bower": meta, "tags": ["bower-play"]})
            if meta["expect"] == "raises":
                cell.metadata["tags"].append("raises-exception")  # nbclient: tolerate the error
            nb.cells.append(cell)
            continue
        # repo block → read-only view with line-anchored source link
        step = steps[b.keys["step"]]
        last_step = step
        shown = shown_lines(b.body)
        path = b.keys["file"]
        start, end = line_range(states[step.id][path], shown)
        tag = f"step-{seq[step.id]:03d}-{step.id}"
        url = repo["links"]["blob"].format(tag=tag, path=path, start=start, end=end)
        expect = b.keys.get("expect", "pass")
        badge = "" if expect == "pass" else f" · **expect: `{expect}`**"
        footer = (f"<sub>`{path}` L{start}–{end} · step {seq[step.id]} of {b.keys['repo']}"
                  f"{badge} · [source]({url})</sub>")
        nb.cells.append(new_markdown_cell(
            f"```{b.lang}\n" + "\n".join(shown) + f"\n```\n{footer}",
            metadata={"bower": {"kind": "view", "step": step.id, "file": path,
                                "lines": [start, end], "expect": expect}}))
    nb.metadata["kernelspec"] = {"name": "python3", "display_name": "Python 3", "language": "python"}
    out.parent.mkdir(parents=True, exist_ok=True)
    nbformat.write(nb, out)
    print(f"wrote {out}: {len(order)} steps, {play_n} play cells, {len(nb.cells)} cells")


# ---------------------------------------------------------------- verify (§ 15.3, open q. 8)

def verify(path: Path) -> int:
    from nbclient import NotebookClient
    nb = nbformat.read(path, as_version=4)
    NotebookClient(nb, timeout=120, allow_errors=True).execute()
    failures = 0
    for i, c in enumerate(nb.cells):
        meta = c.get("metadata", {}).get("bower", {})
        if meta.get("kind") != "play":
            continue
        raised = any(o.get("output_type") == "error" for o in c.get("outputs", []))
        want = meta.get("expect", "pass")
        ok = raised if want == "raises" else not raised
        print(f"  cell {i:2d}  step={meta['step']:<22} expect={want:<7} raised={raised!s:<5} {'ok' if ok else 'FAIL'}")
        failures += not ok
    nbformat.write(nb, path)  # keep executed outputs so the .ipynb reads well cold
    print("verify:", "ok" if not failures else f"{failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build"); b.add_argument("chapter", type=Path); b.add_argument("-o", type=Path, required=True)
    b.add_argument("--config", type=Path, default=Path("book/bower.toml"))
    v = sub.add_parser("verify"); v.add_argument("notebook", type=Path)
    a = ap.parse_args()
    try:
        if a.cmd == "build":
            build(a.chapter, tomllib.loads(a.config.read_text()), a.o)
            return 0
        return verify(a.notebook)
    except PlanError as e:
        print(f"plan error: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
