#!/usr/bin/env python3
"""The harness's counts (CLAUDE.md "Harness"), read from git.

A board line is a list item opening with a bold ask (`- **ask**: …`) in
`docs/boards/*.md`; the ask is its key. A cycle is one line deleted (a key
gone from every board in a commit: a line moved between boards is not a
cycle, nor is an ask reworded in place). The last upkeep is the last commit
carrying an `Upkeep:` trailer.

    python3 scripts/harness.py size      lines per board
    python3 scripts/harness.py age       each line's cycles since it was touched
    python3 scripts/harness.py due       whether an upkeep pass is due
    python3 scripts/harness.py orphans   docs pages docs/index.md does not reach
    python3 scripts/harness.py ratchet   the advisory trailers since the last
        upkeep by class (the ship skill's "Advisory classes"), three of a
        kind marked, and the slugs no class lists
    python3 scripts/harness.py stale <base>|<a>..<b>
        the names a change removes (code identifiers, headings, paths) that
        the pages, CLAUDE.md, the skills and the code's comments still cite,
        the lines it adds to a wrapped page or a whole-line comment past the
        width, and the stub lines a reflow leaves; then, not failing, what to
        re-read: each function whose body it changes with its doc unchanged,
        each page line citing a function it changes or a variant it defines;
        `<base>` alone compares the branch point with the working tree (ship
        step 3)
"""

import datetime
import difflib
import re
import io
import subprocess
import tokenize
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BOARDS = "docs/boards"
KEY = re.compile(r"^- \*\*(.+?)\*\*")
CYCLES_DUE = 20
DAYS_DUE = 30


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def keys(text):
    return {m.group(1) for line in text.splitlines() if (m := KEY.match(line))}


def board_keys(rev):
    """Every board line's key at `rev` (empty before the boards existed)."""
    out = set()
    listing = git("ls-tree", "--name-only", rev, "--", f"{BOARDS}/") if rev else ""
    for path in listing.split():
        if path.endswith(".md"):
            out |= keys(git("show", f"{rev}:{path}"))
    return out


def deleted(before, after):
    """Keys gone from `before`, less those reworded into a new key."""
    added = list(after - before)
    gone = 0
    for key in before - after:
        close = difflib.get_close_matches(key, added, n=1, cutoff=0.6)
        if close:
            added.remove(close[0])
        else:
            gone += 1
    return gone


def board_commits():
    """(sha, date, cycles, upkeep) of each commit touching the boards, oldest first."""
    log = git("log", "--reverse", "--format=%H %cs", "--", BOARDS).split("\n")
    out = []
    for entry in filter(None, log):
        sha, date = entry.split()
        parents = git("rev-list", "--parents", "-n1", sha).split()[1:]
        before = board_keys(parents[0]) if parents else set()
        cycles = deleted(before, board_keys(sha))
        body = git("show", "-s", "--format=%B", sha)
        upkeep = any(line.startswith("Upkeep:") for line in body.splitlines())
        out.append((sha, date, cycles, upkeep))
    return out


def last_upkeep(commits):
    """Index of the last upkeep commit; the first list-shaped board if none."""
    for i in range(len(commits) - 1, -1, -1):
        if commits[i][3]:
            return i
    for i, (sha, *_rest) in enumerate(commits):
        if board_keys(sha):
            return i
    return len(commits) - 1


def size():
    total = 0
    for path in sorted((ROOT / BOARDS).glob("*.md")):
        n = len(keys(path.read_text()))
        total += n
        print(f"{path.stem:<10} {n}")
    print(f"{'total':<10} {total}")


def age():
    commits = board_commits()
    index = {sha: i for i, (sha, *_rest) in enumerate(commits)}
    # Cycles after each board commit, to the tip.
    after = [0] * (len(commits) + 1)
    for i in range(len(commits) - 1, -1, -1):
        after[i] = after[i + 1] + commits[i][2]
    rows = []
    for path in sorted((ROOT / BOARDS).glob("*.md")):
        rel = path.relative_to(ROOT).as_posix()
        blame = git("blame", "--line-porcelain", "HEAD", "--", rel)
        sha = None
        for line in blame.splitlines():
            if re.match(r"^[0-9a-f]{40} ", line):
                sha = line.split()[0]
            elif line.startswith("\t") and (m := KEY.match(line[1:])):
                i = index.get(sha)
                untouched = after[i + 1] if i is not None else 0
                rows.append((untouched, path.stem, m.group(1)))
    for untouched, board, key in sorted(rows, key=lambda r: (-r[0], r[1], r[2])):
        flag = "  on trial" if untouched >= CYCLES_DUE else ""
        print(f"{untouched:>3}  {board:<10} {key}{flag}")


def due():
    commits = board_commits()
    if not commits:
        print("no board history")
        return
    i = last_upkeep(commits)
    cycles = sum(c[2] for c in commits[i + 1 :])
    since = datetime.date.fromisoformat(commits[i][1])
    days = (datetime.date.today() - since).days
    reasons = []
    if cycles >= CYCLES_DUE:
        reasons.append(f"{cycles} cycles")
    if days >= DAYS_DUE:
        reasons.append(f"{days} days")
    word = "due" if reasons else "not due"
    print(
        f"upkeep {word}: {cycles} cycles and {days} days since"
        f" {commits[i][0][:7]} ({since})"
        + (f" — {', '.join(reasons)}" if reasons else "")
    )


def orphans():
    index = (ROOT / "docs/index.md").read_text()
    links = re.findall(r"\]\(([^)#:]+\.md)\)", index)
    linked = {(ROOT / "docs" / link).resolve() for link in links}
    pages = [p for p in git("ls-files", "docs").split() if p.endswith(".md")]
    missing = [
        p
        for p in pages
        if p != "docs/index.md" and (ROOT / p).resolve() not in linked
    ]
    dangling = [link for link in links if not (ROOT / "docs" / link).exists()]
    for p in missing:
        print(f"not in docs/index.md: {p}")
    for link in dangling:
        print(f"docs/index.md links a missing page: {link}")
    if not missing and not dangling:
        print("every docs page reachable from docs/index.md")
    return 1 if missing or dangling else 0


# History: it names what was, by design.
HISTORY = {"CHANGELOG.md"}
# One line an item, never wrapped.
ONE_LINE = ("docs/rules-qa.md", "docs/boards/")
WIDTH = 80
SLASHES = (".rs", ".ts", ".js", ".mjs")
HASHES = (".py", ".sh", ".toml")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
TICKS = re.compile(r"`([^`]*)`")
PATHED = re.compile(r"::([A-Za-z_]\w*)|([A-Za-z_]\w*)::")
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
HEADING = re.compile(r"^#{1,6}\s+(.+?)\s*#*\s*$")


def is_page(path):
    """Every tracked markdown file is a page: the docs, CLAUDE.md, the skills."""
    return path.endswith(".md")


def is_code(path):
    return path.endswith(SLASHES + HASHES)


class Tree:
    """The files at a revision, or in the working tree (`rev` None)."""

    def __init__(self, rev):
        self.rev = rev
        if rev:
            self.paths = git("ls-tree", "-r", "--name-only", rev).split("\n")
        else:
            self.paths = git(
                "ls-files", "--cached", "--others", "--exclude-standard"
            ).split("\n")
        self.paths = [p for p in self.paths if p and (is_page(p) or is_code(p))]
        self.text = {}

    def read(self, path):
        if path not in self.text:
            if self.rev:
                self.text[path] = git("show", f"{self.rev}:{path}")
            else:
                f = ROOT / path
                self.text[path] = f.read_text(errors="replace") if f.is_file() else ""
        return self.text[path]

    def read_all(self, paths):
        """Read `paths` at a revision in one `git cat-file` call."""
        todo = [p for p in paths if p not in self.text]
        if not self.rev or not todo:
            return
        out = subprocess.run(
            ["git", "cat-file", "--batch"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            input="".join(f"{self.rev}:{p}\n" for p in todo).encode(),
        ).stdout
        at = 0
        for p in todo:
            end = out.index(b"\n", at)
            header = out[at:end].split()
            if header[-1] == b"missing":
                self.text[p] = ""
                at = end + 1
                continue
            size = int(header[2])
            self.text[p] = out[end + 1 : end + 1 + size].decode(errors="replace")
            at = end + 1 + size + 1


def split_comments(path, text):
    """(code, comment) of each line of a code file."""
    lines = text.split("\n")
    if path.endswith(SLASHES):
        out = []
        for line in lines:
            code, sep, comment = line.partition("//")
            out.append((code, comment))
        return out
    if not path.endswith(".py"):
        return [(c, m) for c, _s, m in (line.partition("#") for line in lines)]
    # Python: comments and docstrings, by its own tokenizer.
    out = [[line, ""] for line in lines]
    try:
        toks = list(tokenize.generate_tokens(io.StringIO(text).readline))
    except (tokenize.TokenError, SyntaxError, IndentationError):
        return [(c, m) for c, _s, m in (line.partition("#") for line in lines)]
    prev = None
    for tok in toks:
        if tok.type == tokenize.COMMENT:
            row = tok.start[0] - 1
            out[row][1] += tok.string
            out[row][0] = out[row][0].replace(tok.string, "")
        elif tok.type == tokenize.STRING and prev in (
            None,
            tokenize.INDENT,
            tokenize.DEDENT,
            tokenize.NEWLINE,
            tokenize.NL,
        ):
            for row in range(tok.start[0] - 1, tok.end[0]):
                out[row][1] += out[row][0]
                out[row][0] = ""
        if tok.type not in (tokenize.COMMENT, tokenize.NL):
            prev = tok.type
    return [tuple(pair) for pair in out]


def endpoints(spec):
    """(start revision, end revision or None for the working tree)."""
    a, dots, b = spec.partition("..")
    if dots:
        b = b or "HEAD"
        return git("merge-base", a, b).strip(), b
    return git("merge-base", a, "HEAD").strip(), None


def diff_lines(start, end):
    """(removed, added): (path, old line number, line) removed, (path, line
    number, line) added."""
    args = ["diff", "-U0", "--no-color", "--no-ext-diff", "-M", start]
    args += [end] if end else []
    removed, added = [], []
    old = new = None
    row = orow = 0
    for line in git(*args).split("\n"):
        if line.startswith("--- "):
            old = line[6:] if line.startswith("--- a/") else None
        elif line.startswith("+++ "):
            new = line[6:] if line.startswith("+++ b/") else None
        elif line.startswith("@@"):
            row = int(re.search(r"\+(\d+)", line).group(1))
            orow = int(re.search(r"-(\d+)", line).group(1))
        elif line.startswith("-") and old:
            removed.append((old, orow, line[1:]))
            orow += 1
        elif line.startswith("+") and new:
            added.append((new, row, line[1:]))
            row += 1
    if not end:
        for path in git("ls-files", "--others", "--exclude-standard").split("\n"):
            if path and is_page(path):
                text = (ROOT / path).read_text(errors="replace")
                added += [(path, i, t) for i, t in enumerate(text.split("\n"), 1)]
    return removed, added


def shaped(name):
    """A name that reads as code even outside backticks and paths (a
    capitalized word may only open a sentence)."""
    return "_" in name or any(c.isupper() for c in name[1:])


def slug(text):
    return re.sub(r"[^\w\- ]", "", text.lower()).replace(" ", "-")


def heading_texts(text):
    out = set()
    for line in text.split("\n"):
        if m := HEADING.match(line):
            out.add(m.group(1))
            out.add(re.sub(r"^§?[\d.]+\s+", "", m.group(1)))
    return out


def removed_names(start, end, tree, removed):
    """{name: why} of what the change removes."""
    names = {}
    # Code identifiers no code file has any more.
    corpus = set()
    code_files = [p for p in tree.paths if is_code(p)]
    tree.read_all(code_files)
    for path in code_files:
        for code, _comment in split_comments(path, tree.read(path)):
            corpus.update(WORD.findall(code))
    # A removed line read within its old file, where a docstring is seen
    # whole: its words are no code names.
    old = Tree(start)
    old.read_all({p for p, _row, _line in removed if is_code(p)})
    olds = {}
    for path, row, _line in removed:
        if is_code(path):
            if path not in olds:
                olds[path] = split_comments(path, old.read(path))
            code = STRING.sub("", olds[path][row - 1][0])
            for name in WORD.findall(code):
                if len(name) >= 4 and name not in corpus:
                    names[("word", name)] = "gone from the code"
    # Headings gone from their page.
    renames = {}
    status = ["diff", "--name-status", "-M", start] + ([end] if end else [])
    gone_paths = []
    for entry in git(*status).split("\n"):
        parts = entry.split("\t")
        if parts[0].startswith("R"):
            renames[parts[1]] = parts[2]
            gone_paths.append(parts[1])
        elif parts[0] == "D":
            gone_paths.append(parts[1])
    for path, _row, line in removed:
        if path.endswith(".md") and (m := HEADING.match(line)):
            now = heading_texts(tree.read(renames.get(path, path)))
            bare = re.sub(r"^§?[\d.]+\s+", "", m.group(1))
            if m.group(1) not in now and bare not in now:
                for text in {m.group(1), bare}:
                    names[("heading", text)] = f"a heading gone from {path}"
    # Paths gone, and the directories they leave empty.
    after = set(
        git("ls-tree", "-r", "--name-only", end).split("\n")
        if end
        else git("ls-files", "--cached", "--others", "--exclude-standard").split(
            "\n"
        )
    )
    after_dirs = {str(d) for p in after for d in Path(p).parents}
    for path in gone_paths:
        names[("path", path)] = "a path gone"
        if not any(p.endswith("/" + path.split("/", 2)[-1]) for p in after):
            names[("path", path.split("/", 2)[-1])] = f"a path gone ({path})"
        for d in Path(path).parents:
            d = str(d)
            if d != "." and d not in after_dirs and "/" in d:
                names[("path", d + "/")] = "a directory gone"
    return names


def citations(names, tree):
    """(name, why, path, line number) of each citation left."""
    words = {n: why for (kind, n), why in names.items() if kind == "word"}
    quoted = {n: why for (kind, n), why in names.items() if kind == "heading"}
    paths = {n: why for (kind, n), why in names.items() if kind == "path"}
    anchors = {"#" + slug(n): (n, why) for n, why in quoted.items()}
    found = []
    pages = [p for p in tree.paths if is_page(p) and p not in HISTORY]
    code = [p for p in tree.paths if is_code(p)]
    tree.read_all(pages + code)
    for path in pages + code:
        text = tree.read(path)
        if path in pages:
            lines = text.split("\n")
        else:
            lines = [comment for _code, comment in split_comments(path, text)]
        for i, line in enumerate(lines, 1):
            if not line:
                continue
            ticked = " ".join(TICKS.findall(line))
            here = set(WORD.findall(line))
            ticked_words = set(WORD.findall(ticked))
            ticked_words.update(w for pair in PATHED.findall(line) for w in pair)
            for n in here & words.keys():
                if shaped(n) or n in ticked_words:
                    found.append((f"`{n}`", words[n], path, i))
            for n, why in quoted.items():
                if f'"{n}"' in line:
                    found.append((f'"{n}"', why, path, i))
            for a, (n, why) in anchors.items():
                if a + ")" in line:
                    found.append((f'"{n}"', why, path, i))
            for n, why in paths.items():
                if re.search(r"(?<![\w/.-])" + re.escape(n) + r"(?![\w/-])", line):
                    found.append((n, why, path, i))
    return found


def too_wide(added, tree):
    """(path, line number, columns) of each added line of a wrapped page past
    the width; code blocks, tables, headings and URLs aside."""
    out = []
    for path in sorted({p for p, _i, _t in added}):
        if not is_page(path) or path.startswith(ONE_LINE):
            continue
        rows = {i for p, i, _t in added if p == path}
        fence = False
        for i, line in enumerate(tree.read(path).split("\n"), 1):
            if line.lstrip().startswith(("```", "~~~")):
                fence = not fence
                continue
            if (
                i in rows
                and not fence
                and len(line) > WIDTH
                and not line.lstrip().startswith(("|", "#"))
                and "://" not in line
            ):
                out.append((path, i, len(line)))
    return out


LINE_COMMENT = re.compile(r"^\s*(///|//!|//)(?: |$)")
HASH_COMMENT = re.compile(r"^\s*(#)(?: |$)")
LIST_START = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s")


def comment_prefix(path, line):
    """The line-comment marker a whole-line comment opens with, else None."""
    pattern = LINE_COMMENT if path.endswith(SLASHES) else HASH_COMMENT
    m = pattern.match(line) if is_code(path) else None
    return m.group(1) if m else None


def paragraphs(path, lines):
    """Runs of (line number, line, its text without its marker) wrapped as
    one paragraph: a page's prose or a quote's (`>` its marker), or a block
    of whole-line comments sharing a marker. Fences, tables, headings, front
    matter, list starts, blank lines (a bare marker too) and hard breaks end
    a run."""
    runs, run, fence, front = [], [], False, False
    marker = None
    for i, line in enumerate(lines, 1):
        if is_page(path):
            if i == 1 and line == "---":
                front = True
                continue
            if front:
                front = line != "---"
                continue
            quote = re.match(r"^\s*(>+) ?", line)
            mark = quote.group(1) if quote else None
            text = line[quote.end() :] if quote else line
        else:
            mark = comment_prefix(path, line)
            text = line.split(mark, 1)[1] if mark else ""
        if text.lstrip().startswith(("```", "~~~")):
            fence = not fence
        breaks = (
            fence
            or not text.strip()
            or text.lstrip().startswith(("```", "~~~", "|", "#", "<"))
            or mark != marker
        )
        if breaks or LIST_START.match(text):
            if run:
                runs.append(run)
            run = []
        marker = mark
        if not breaks:
            run.append((i, line, text))
        if line.endswith(("  ", "\\")) and run:
            runs.append(run)
            run = []
    if run:
        runs.append(run)
    return runs


def stubs(added, tree):
    """(path, line number) of each line a reflow left short, next to a line
    the change adds, in a paragraph of three lines or more: not its last
    line, under three quarters of the paragraph's second-longest line, the
    next line's first word fitting on it."""
    out = []
    for path in sorted({p for p, _i, _t in added}):
        if path in HISTORY or path.startswith(ONE_LINE) or not (
            is_page(path) or is_code(path)
        ):
            continue
        rows = {i for p, i, _t in added if p == path}
        for run in paragraphs(path, tree.read(path).split("\n")):
            if len(run) < 3:
                continue
            widths = sorted((len(t) for _i, t, _x in run), reverse=True)
            for (i, line, _x), (j, _after, text) in zip(run, run[1:]):
                if i not in rows and j not in rows:
                    continue
                # A code span is one word, never broken across lines: a
                # line leaving one open is no stub.
                if line.count("`") % 2:
                    continue
                rest = text.lstrip()
                word = rest.split()[0]
                if word.count("`") % 2:
                    close = rest.find("`", word.find("`") + 1)
                    word = rest if close < 0 else rest[: close + 1]
                # A sentence ended before a break is a writer's choice.
                if "://" in line + word or line.rstrip().endswith(tuple(".:;?!")):
                    continue
                if len(line) < widths[1] * 3 / 4 and len(line) + 1 + len(
                    word
                ) <= min(widths[0], WIDTH):
                    out.append((path, i))
    return out


def wide_comments(added):
    """(path, line number, columns) of each added whole-line comment past the
    width."""
    return [
        (p, i, len(t))
        for p, i, t in added
        if len(t) > WIDTH and comment_prefix(p, t) and "://" not in t
    ]


RUST_FN = re.compile(
    r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)"
)
PY_DEF = re.compile(r"^(\s*)(?:async\s+)?def\s+(\w+)")
RUST_ENUM = re.compile(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?enum\s+\w+")
VARIANT = re.compile(r"^(\s+)([A-Z][A-Za-z0-9]+)\b\s*(?:[,({=]|$)")


def functions(path, text):
    """[(name, doc rows, body rows)] of each function in a Rust or Python
    file, in order; a doc is the `///` lines over a Rust fn (attributes
    between) or a Python def's docstring."""
    lines = text.split("\n")
    pattern = RUST_FN if path.endswith(".rs") else PY_DEF
    out = []
    for n, line in enumerate(lines, 1):
        m = pattern.match(line)
        if not m:
            continue
        indent = len(m.group(1))
        end = n
        # A one-line fn, or a trait's without a body, ends on its line.
        one = path.endswith(".rs") and line.rstrip().endswith(("}", ";"))
        for k in range(n + 1, 0 if one else len(lines) + 1):
            row = lines[k - 1]
            if not row.strip():
                continue
            if len(row) - len(row.lstrip()) <= indent and (
                path.endswith(".py") or row.strip().startswith("}")
            ):
                end = k if path.endswith(".rs") else k - 1
                break
            end = k
        doc = set()
        if path.endswith(".rs"):
            k = n - 1
            while k >= 1 and lines[k - 1].strip().startswith(("///", "#[")):
                if lines[k - 1].strip().startswith("///"):
                    doc.add(k)
                k -= 1
        else:
            k = n
            while k <= end and not lines[k - 1].rstrip().endswith(":"):
                k += 1
            first = k + 1
            if first <= end and lines[first - 1].strip().startswith(('"""', "'''")):
                quote = lines[first - 1].strip()[:3]
                last = first
                while last <= end and not (
                    lines[last - 1].rstrip().endswith(quote)
                    and (last > first or lines[first - 1].strip().count(quote) > 1)
                ):
                    last += 1
                doc = set(range(first, min(last, end) + 1))
        out.append((m.group(2), doc, set(range(n, end + 1)) - doc))
    return out


def keyed(entries):
    """{(name, occurrence): (doc, body)}: a name defined twice in a file (two
    impls' `new`) is two functions."""
    seen, out = {}, {}
    for name, doc, body in entries:
        seen[name] = seen.get(name, -1) + 1
        out[(name, seen[name])] = (doc, body)
    return out


def variants(path, text, rows):
    """The enum variants a Rust file defines on `rows`."""
    lines = text.split("\n")
    out = set()
    for r in rows:
        m = VARIANT.match(lines[r - 1]) if r <= len(lines) else None
        if not m:
            continue
        for k in range(r - 1, 0, -1):
            above = lines[k - 1]
            if above.strip() and len(above) - len(above.lstrip()) < len(m.group(1)):
                if RUST_ENUM.match(above):
                    out.add(m.group(2))
                break
    return out


def reread(start, end, tree, removed, added):
    """What a change leaves to re-read: (path, line, note) of each function
    whose body it changes with its doc unchanged, and of each page line
    citing in backticks a function it changes or a variant it defines."""
    changed = {}  # (path, (name, occurrence)) -> [body changed, doc changed]
    names = set()
    old = Tree(start)
    for side, lines, at in (("old", removed, old), ("new", added, tree)):
        by_path = {}
        for path, row, _t in lines:
            if path.endswith((".rs", ".py")):
                by_path.setdefault(path, set()).add(row)
        for path, rows in by_path.items():
            text = at.read(path)
            for key, (doc, body) in keyed(functions(path, text)).items():
                hit = changed.setdefault((path, key), [False, False])
                hit[0] |= bool(rows & body)
                hit[1] |= bool(rows & doc)
            if path.endswith(".rs"):
                names |= variants(path, text, rows)
    # A name defined more than once in the code (`main`, `new`) is no
    # citation of this one.
    defined = {}
    code = [p for p in tree.paths if p.endswith((".rs", ".py"))]
    tree.read_all(code)
    for path in code:
        pattern = RUST_FN if path.endswith(".rs") else PY_DEF
        for line in tree.read(path).split("\n"):
            if m := pattern.match(line):
                defined[m.group(2)] = defined.get(m.group(2), 0) + 1
    out = []
    docs = {}
    for (path, (name, nth)), (body, doc) in sorted(changed.items()):
        if not body:
            continue
        if len(name) >= 4 and defined.get(name, 0) <= 1:
            names.add(name)
        if path not in docs:
            docs[path] = (
                keyed(functions(path, tree.read(path))) if path in tree.paths else {}
            )
        here = docs[path].get((name, nth))
        if here and here[0] and not doc:
            out.append((path, min(here[1]), f"fn {name}: body changed, doc unchanged"))
    touched = {(p, i) for p, i, _t in added}
    pages = [p for p in tree.paths if is_page(p) and p not in HISTORY]
    tree.read_all(pages)
    for path in pages:
        for i, line in enumerate(tree.read(path).split("\n"), 1):
            if (path, i) in touched or "`" not in line:
                continue
            # The token is the name: `f`, `f()`, `m::f`, `X::V`, `s.f`.
            ticked = {
                re.sub(r"\(.*$", "", t).replace("::", ".").split(".")[-1]
                for t in TICKS.findall(line)
                if re.fullmatch(r"[\w:.]+(?:\(.*\))?", t)
            }
            for n in sorted(ticked & names):
                out.append((path, i, f"cites `{n}`, changed"))
    return out


def stale():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    start, end = endpoints(sys.argv[2])
    tree = Tree(end)
    removed, added = diff_lines(start, end)
    found = citations(removed_names(start, end, tree, removed), tree)
    for name, why, path, i in sorted(set(found), key=lambda f: (f[2], f[3])):
        print(f"{path}:{i}: {name} ({why})")
    wide = too_wide(added, tree) + wide_comments(added)
    for path, i, columns in wide:
        print(f"{path}:{i}: {columns} columns, past {WIDTH}")
    short = stubs(added, tree)
    for path, i in short:
        print(f"{path}:{i}: a stub line (the next line's first word fits on it)")
    if not found and not wide and not short:
        print("no removed name cited, no added line past the width, no stub")
    notes = reread(start, end, tree, removed, added)
    if notes:
        print(f"to re-read ({len(notes)}, not failing): still true after the change?")
        for path, i, note in notes:
            print(f"  {path}:{i}: {note}")
    return 1 if found or wide or short else 0


SHIP = ".claude/skills/ship/SKILL.md"


def classes():
    """(exact, prefixes): each slug, each `prefix-*`, to its class, from the
    ship skill's "Advisory classes" (a class's line: its slug first)."""
    text = (ROOT / SHIP).read_text()
    part = text.split("### Advisory classes", 1)[1]
    exact, prefixes = {}, {}
    kind = None
    for line in part.split("\n"):
        if line.startswith("- `"):
            kind = re.match(r"- `([^`]+)`", line).group(1)
        elif not (kind and line.startswith("  ")):
            # The list ends at its first line that is not a class's.
            if exact and line.strip():
                break
            kind = None
            continue
        for slug in re.findall(r"`([^`]+)`", line):
            if slug.endswith("*"):
                prefixes[slug[:-1]] = kind
            else:
                exact[slug] = kind
    return exact, prefixes


def classify(slug, exact, prefixes):
    if slug in exact:
        return exact[slug]
    hits = [p for p in prefixes if slug.startswith(p)]
    return prefixes[max(hits, key=len)] if hits else None


def ratchet():
    commits = board_commits()
    since = commits[last_upkeep(commits)][0] if commits else None
    log = git("log", "--format=%B", f"{since}..HEAD" if since else "HEAD")
    exact, prefixes = classes()
    counts, unlisted = {}, {}
    for line in log.splitlines():
        if m := re.match(r"^Advisory: ([a-z0-9-]+)", line):
            kind = classify(m.group(1), exact, prefixes)
            if kind:
                counts[kind] = counts.get(kind, 0) + 1
            else:
                unlisted[m.group(1)] = unlisted.get(m.group(1), 0) + 1
    print(f"advisories since {since[:7] if since else 'the start'}, by class:")
    for kind, n in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"{n:>4}  {kind}{'  three of a kind' if n >= 3 else ''}")
    if unlisted:
        print("no class lists:", ", ".join(f"{k} ({n})" for k, n in sorted(unlisted.items())))


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    commands = {
        "size": size,
        "age": age,
        "due": due,
        "orphans": orphans,
        "ratchet": ratchet,
        "stale": stale,
    }
    if cmd not in commands:
        sys.exit(__doc__)
    sys.exit(commands[cmd]() or 0)
