#!/usr/bin/env python3
"""E1 — Anchor survival simulation (optimized).

READ-ONLY: no commits to any repo. Only reads git history.

Optimizations vs naive approach:
- Only check anchors when their file actually changed
- Use git grep instead of scanning all files
- 20 anchors per repo, 20 commits forward
- Cache file contents at each commit
"""

import subprocess
import re
import os
import sys
import json
from collections import defaultdict
from dataclasses import dataclass
from typing import Optional

REPOS = [
    ("kiseki", "/home/witlox/src/kiseki"),
    ("yoyo-evolve", "/tmp/opencode/yoyo-evolve-full"),
    ("tokio", "/tmp/opencode/tokio"),
    ("cobra", "/tmp/opencode/cobra"),
    ("httpx", "/tmp/opencode/httpx"),
]

START_OFFSET = 50
REPLAY_N = 20
ANCHORS_PER_REPO = 20

SOURCE_EXTS = {".rs": "rust", ".go": "go", ".py": "python", ".ts": "typescript", ".js": "javascript"}
SKIP_DIRS = ["target/", "vendor/", "node_modules/", ".git/", "testdata/", "fixtures/"]

SYMBOL_PATTERNS = {
    "rust": [(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)", "fn"), (r"^\s*(pub\s+)?struct\s+(\w+)", "struct"), (r"^\s*(pub\s+)?enum\s+(\w+)", "enum"), (r"^\s*(pub\s+)?trait\s+(\w+)", "trait")],
    "go": [(r"^\s*func\s+(\w+)", "func"), (r"^\s*func\s*\([^)]*\)\s+(\w+)", "func"), (r"^\s*type\s+(\w+)\s+", "type")],
    "python": [(r"^\s*(async\s+)?def\s+(\w+)", "def"), (r"^\s*class\s+(\w+)", "class")],
    "typescript": [(r"^\s*(export\s+)?(async\s+)?function\s+(\w+)", "function"), (r"^\s*(export\s+)?class\s+(\w+)", "class"), (r"^\s*(export\s+)?interface\s+(\w+)", "interface")],
    "javascript": [(r"^\s*(export\s+)?(async\s+)?function\s+(\w+)", "function"), (r"^\s*(export\s+)?class\s+(\w+)", "class")],
}


@dataclass
class Anchor:
    path: str
    start_line: int
    end_line: int
    symbol: str
    content: str
    normalized: str
    language: str


def git(repo, *args):
    r = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)
    return r.stdout, r.returncode


def get_commits(repo, n):
    out, _ = git(repo, "log", f"--format=%H", f"-{n}")
    return [c for c in out.strip().split("\n") if c]


def list_source_files(repo, commit):
    out, _ = git(repo, "ls-tree", "-r", "--name-only", commit)
    files = []
    for f in out.strip().split("\n"):
        if not f:
            continue
        ext = os.path.splitext(f)[1]
        if ext not in SOURCE_EXTS:
            continue
        if any(s in f for s in SKIP_DIRS):
            continue
        files.append(f)
    return files


def read_file_at(repo, commit, path):
    out, code = git(repo, "show", f"{commit}:{path}")
    if code != 0:
        return None
    return out


def normalize(content):
    result = []
    for line in content.split("\n"):
        line = re.sub(r"//.*$", "", line)
        line = re.sub(r"#.*$", "", line)
        line = re.sub(r"/\*.*?\*/", "", line)
        line = re.sub(r"\s+", "", line)
        if line:
            result.append(line)
    return "".join(result)


def extract_anchors(repo, commit):
    anchors = []
    files = list_source_files(repo, commit)

    # Sample evenly across files to get diverse anchors
    if len(files) > ANCHORS_PER_REPO * 3:
        step = len(files) // (ANCHORS_PER_REPO * 3)
        files = files[::step]

    for path in files:
        ext = os.path.splitext(path)[1]
        lang = SOURCE_EXTS.get(ext)
        if not lang:
            continue

        content = read_file_at(repo, commit, path)
        if not content:
            continue

        lines = content.split("\n")
        patterns = SYMBOL_PATTERNS.get(lang, [])

        for i, line in enumerate(lines):
            for pattern, kind in patterns:
                m = re.match(pattern, line)
                if m:
                    symbol = m.groups()[-1]
                    indent = len(line) - len(line.lstrip())
                    end = min(i + 20, len(lines) - 1)
                    for j in range(i + 1, min(i + 50, len(lines))):
                        nl = lines[j]
                        if nl.strip() == "":
                            continue
                        ni = len(nl) - len(nl.lstrip())
                        if ni <= indent and nl.strip():
                            end = j - 1
                            break
                    else:
                        end = min(i + 20, len(lines) - 1)

                    span = "\n".join(lines[i : end + 1])
                    norm = normalize(span)

                    anchors.append(Anchor(path=path, start_line=i + 1, end_line=end + 1, symbol=symbol, content=span, normalized=norm, language=lang))
                    break

            if len(anchors) >= ANCHORS_PER_REPO:
                break
        if len(anchors) >= ANCHORS_PER_REPO:
            break

    return anchors


def get_changed_files(repo, prev, curr):
    out, _ = git(repo, "diff", "--name-status", f"{prev}..{curr}")
    changes = {}
    for line in out.strip().split("\n"):
        if not line:
            continue
        parts = line.split("\t")
        status = parts[0]
        path = parts[-1] if len(parts) > 1 else ""
        changes[path] = status
    return changes


def classify_commit(changes):
    types = set()
    for path, status in changes.items():
        if status.startswith("R"):
            types.add("rename")
        elif status.startswith("D"):
            types.add("delete")
        elif status.startswith("A"):
            types.add("add")
        elif status.startswith("M"):
            # Check commit message for format/fmt indicators
            types.add("semantic")  # Default to semantic, refined below
    if "delete" in types and len(types) == 1:
        return "delete"
    if "rename" in types and "semantic" not in types:
        return "rename"
    if not types:
        return "none"
    if "semantic" in types:
        return "semantic"
    return "other"


def classify_commit_with_msg(repo, prev, curr, changes):
    base = classify_commit(changes)
    if base != "semantic":
        return base

    # Check commit message for reformat indicators
    msg, _ = git(repo, "log", "--format=%s", "-1", curr)
    msg_lower = msg.lower().strip() if msg else ""
    if any(kw in msg_lower for kw in ["fmt", "format", "gofmt", "rustfmt", "black", "prettier", "lint", "style", "whitespace"]):
        return "reformat"

    # Check if changes are mostly whitespace
    for path, status in changes.items():
        if status.startswith("M"):
            diff_w, _ = git(repo, "diff", "-w", "--numstat", f"{prev}..{curr}", "--", path)
            diff_full, _ = git(repo, "diff", "--numstat", f"{prev}..{curr}", "--", path)
            # If -w diff shows 0 changes but full diff shows changes, it's a reformat
            if diff_w.strip() == "" and diff_full.strip():
                return "reformat"

    return "semantic"


def resolve_path_range(anchor, content_at_curr):
    """Path-range: same path, same line range."""
    if content_at_curr is None:
        return "failed"
    lines = content_at_curr.split("\n")
    if anchor.end_line > len(lines) or anchor.start_line > len(lines):
        return "failed"
    span = "\n".join(lines[anchor.start_line - 1 : anchor.end_line])
    if span == anchor.content:
        return "correct"
    return "wrong"


def resolve_symbol(anchor, repo, curr, content_at_curr):
    """Symbol: find the symbol definition in the tree."""
    if content_at_curr is not None:
        # File still exists. Check if symbol is still there.
        lines = content_at_curr.split("\n")
        patterns = SYMBOL_PATTERNS.get(anchor.language, [])
        for i, line in enumerate(lines):
            for pattern, kind in patterns:
                m = re.match(pattern, line)
                if m and m.groups()[-1] == anchor.symbol:
                    # Found the symbol. Check if content matches.
                    indent = len(line) - len(line.lstrip())
                    end = min(i + 20, len(lines) - 1)
                    for j in range(i + 1, min(i + 50, len(lines))):
                        nl = lines[j]
                        if nl.strip() == "":
                            continue
                        ni = len(nl) - len(nl.lstrip())
                        if ni <= indent and nl.strip():
                            end = j - 1
                            break
                    span = "\n".join(lines[i : end + 1])
                    if span == anchor.content:
                        return "correct"
                    return "wrong"  # Symbol found but content changed

    # File doesn't exist or symbol not in original file. Search elsewhere.
    out, code = git(repo, "grep", "-l", "--", anchor.symbol, curr)
    if code != 0 or not out.strip():
        return "failed"

    # Check each matching file
    found_correct = False
    found_wrong = False
    for path in out.strip().split("\n"):
        if not path:
            continue
        # git grep returns: <commit>:<path>
        if ":" in path:
            path = path.split(":", 1)[1] if path.startswith(curr) else path.split(":", 1)[-1]

        ext = os.path.splitext(path)[1]
        if SOURCE_EXTS.get(ext) != anchor.language:
            continue

        content = read_file_at(repo, curr, path)
        if not content:
            continue

        lines = content.split("\n")
        patterns = SYMBOL_PATTERNS.get(anchor.language, [])
        for i, line in enumerate(lines):
            for pattern, kind in patterns:
                m = re.match(pattern, line)
                if m and m.groups()[-1] == anchor.symbol:
                    indent = len(line) - len(line.lstrip())
                    end = min(i + 20, len(lines) - 1)
                    for j in range(i + 1, min(i + 50, len(lines))):
                        nl = lines[j]
                        if nl.strip() == "":
                            continue
                        ni = len(nl) - len(nl.lstrip())
                        if ni <= indent and nl.strip():
                            end = j - 1
                            break
                    span = "\n".join(lines[i : end + 1])
                    if span == anchor.content:
                        found_correct = True
                    else:
                        found_wrong = True
                    break

    if found_correct and not found_wrong:
        return "correct"
    if found_wrong and not found_correct:
        return "wrong"
    if found_correct and found_wrong:
        return "wrong"  # Ambiguous
    return "failed"


def resolve_content_digest(anchor, repo, curr, content_at_curr):
    """Content-digest: find the normalized content anywhere in the tree."""
    if not anchor.normalized or len(anchor.normalized) < 20:
        return "failed"

    if content_at_curr is not None:
        # Check if the original file still contains the anchor
        lines = content_at_curr.split("\n")
        patterns = SYMBOL_PATTERNS.get(anchor.language, [])
        for i, line in enumerate(lines):
            for pattern, kind in patterns:
                m = re.match(pattern, line)
                if m:
                    indent = len(line) - len(line.lstrip())
                    end = min(i + 20, len(lines) - 1)
                    for j in range(i + 1, min(i + 50, len(lines))):
                        nl = lines[j]
                        if nl.strip() == "":
                            continue
                        ni = len(nl) - len(nl.lstrip())
                        if ni <= indent and nl.strip():
                            end = j - 1
                            break
                    span = "\n".join(lines[i : end + 1])
                    norm = normalize(span)
                    if norm == anchor.normalized:
                        return "correct"

    # Search other files. Use git grep with a unique substring.
    # Take a 40-char unique substring from the normalized content.
    target = anchor.normalized
    if len(target) > 40:
        search_str = target[:40]
    else:
        search_str = target

    # Use git grep to find files containing the search string
    # This is much faster than reading every file
    out, code = git(repo, "grep", "-l", "--", search_str, curr)
    if code != 0 or not out.strip():
        return "failed"

    matches = []
    for path in out.strip().split("\n"):
        if not path:
            continue
        if ":" in path:
            path = path.split(":", 1)[1] if path.startswith(curr) else path.split(":", 1)[-1]

        ext = os.path.splitext(path)[1]
        if SOURCE_EXTS.get(ext) != anchor.language:
            continue

        content = read_file_at(repo, curr, path)
        if not content:
            continue

        lines = content.split("\n")
        patterns = SYMBOL_PATTERNS.get(anchor.language, [])
        for i, line in enumerate(lines):
            for pattern, kind in patterns:
                m = re.match(pattern, line)
                if m:
                    indent = len(line) - len(line.lstrip())
                    end = min(i + 20, len(lines) - 1)
                    for j in range(i + 1, min(i + 50, len(lines))):
                        nl = lines[j]
                        if nl.strip() == "":
                            continue
                        ni = len(nl) - len(nl.lstrip())
                        if ni <= indent and nl.strip():
                            end = j - 1
                            break
                    span = "\n".join(lines[i : end + 1])
                    norm = normalize(span)
                    if norm == anchor.normalized:
                        matches.append(path)
                    break

    if len(matches) == 1:
        return "correct"
    if len(matches) > 1:
        return "wrong"  # Ambiguous
    return "failed"


def run_simulation(repo_name, repo_path):
    print(f"\n{'='*60}")
    print(f"E1: {repo_name} ({repo_path})")
    print(f"{'='*60}")

    commits = get_commits(repo_path, START_OFFSET + REPLAY_N + 10)
    if len(commits) < START_OFFSET + REPLAY_N:
        print(f"  Not enough commits ({len(commits)}), skipping")
        return None

    t0 = commits[START_OFFSET]
    replay = list(reversed(commits[:START_OFFSET]))[:REPLAY_N]

    print(f"  T0: {t0[:12]}, replay: {len(replay)} commits")

    anchors = extract_anchors(repo_path, t0)
    print(f"  Anchors: {len(anchors)}")

    if len(anchors) < 5:
        print(f"  Too few anchors, skipping")
        return None

    # results: strategy -> commit_type -> {correct, wrong, failed, total}
    results = {s: defaultdict(lambda: {"correct": 0, "wrong": 0, "failed": 0, "total": 0})
               for s in ["path-range", "symbol", "content-digest", "multi"]}

    # Cache: file content at each commit (only for changed files)
    content_cache = {}

    prev = t0
    for i, commit in enumerate(replay):
        changes = get_changed_files(repo_path, prev, commit)
        commit_type = classify_commit_with_msg(repo_path, prev, commit, changes)

        if commit_type == "none" or not changes:
            prev = commit
            continue

        # For each anchor, only check if its file was affected
        for anchor in anchors:
            file_changed = anchor.path in changes or any(
                anchor.path in v for v in changes.values()
            )

            # If file unchanged, all strategies still resolve correctly
            if not file_changed:
                for strat in results:
                    results[strat][commit_type]["correct"] += 1
                    results[strat][commit_type]["total"] += 1
                continue

            # File changed — read the new content
            new_status = changes.get(anchor.path, "")
            if new_status.startswith("D") or new_status.startswith("R"):
                content_at_curr = None  # File gone or renamed
                if new_status.startswith("R"):
                    # Find the new name
                    for path, status in changes.items():
                        if status.startswith("R") and path == anchor.path:
                            content_at_curr = read_file_at(repo_path, commit, path)
                            break
                    else:
                        # The rename target might be in the changes
                        for path, status in changes.items():
                            if status.startswith("R") and anchor.path in status:
                                content_at_curr = read_file_at(repo_path, commit, path)
                                break
                        else:
                            content_at_curr = None
            else:
                content_at_curr = read_file_at(repo_path, commit, anchor.path)

            # Path-range
            pr = resolve_path_range(anchor, content_at_curr)
            results["path-range"][commit_type][pr] += 1
            results["path-range"][commit_type]["total"] += 1

            # Symbol
            sym = resolve_symbol(anchor, repo_path, commit, content_at_curr)
            results["symbol"][commit_type][sym] += 1
            results["symbol"][commit_type]["total"] += 1

            # Content-digest
            cd = resolve_content_digest(anchor, repo_path, commit, content_at_curr)
            results["content-digest"][commit_type][cd] += 1
            results["content-digest"][commit_type]["total"] += 1

            # Multi: try all three, report degraded if disagreement
            multi_correct = sum(1 for r in [pr, sym, cd] if r == "correct")
            multi_wrong = sum(1 for r in [pr, sym, cd] if r == "wrong")

            if multi_wrong > 0:
                multi_result = "wrong"
            elif multi_correct >= 2:
                multi_result = "correct"
            elif multi_correct == 1:
                multi_result = "correct"  # One correct, rest failed (no disagreement)
            else:
                multi_result = "failed"

            results["multi"][commit_type][multi_result] += 1
            results["multi"][commit_type]["total"] += 1

        prev = commit
        print(f"  [{i+1}/{len(replay)}] {commit_type} ({len(changes)} files)")

    return results


def print_report(all_results):
    print(f"\n{'='*60}")
    print("E1 — ANCHOR SURVIVAL REPORT")
    print(f"{'='*60}")

    agg = {s: {"correct": 0, "wrong": 0, "failed": 0, "total": 0} for s in ["path-range", "symbol", "content-digest", "multi"]}
    by_type = {s: defaultdict(lambda: {"correct": 0, "wrong": 0, "failed": 0, "total": 0}) for s in ["path-range", "symbol", "content-digest", "multi"]}

    for repo_name, results in all_results.items():
        if results is None:
            continue
        for strat, type_buckets in results.items():
            for commit_type, bucket in type_buckets.items():
                for k in ["correct", "wrong", "failed", "total"]:
                    agg[strat][k] += bucket.get(k, 0)
                    by_type[strat][commit_type][k] += bucket.get(k, 0)

    print("\n--- Aggregate per strategy ---\n")
    print(f"{'Strategy':<20} {'Correct':>8} {'Wrong':>8} {'Failed':>8} {'Total':>8} {'Corr%':>7} {'Wrong%':>7}")
    print("-" * 75)
    for strat in ["path-range", "symbol", "content-digest", "multi"]:
        a = agg[strat]
        if a["total"] == 0:
            continue
        corr_pct = 100 * a["correct"] / a["total"]
        wrong_pct = 100 * a["wrong"] / a["total"]
        print(f"{strat:<20} {a['correct']:>8} {a['wrong']:>8} {a['failed']:>8} {a['total']:>8} {corr_pct:>6.1f}% {wrong_pct:>6.1f}%")

    print("\n--- Breakdown by refactor class ---\n")
    for strat in ["path-range", "symbol", "content-digest", "multi"]:
        print(f"\n  {strat}:")
        for ct in ["rename", "delete", "reformat", "semantic", "add", "other"]:
            b = by_type[strat].get(ct)
            if not b or b["total"] == 0:
                continue
            cp = 100 * b["correct"] / b["total"]
            wp = 100 * b["wrong"] / b["total"]
            print(f"    {ct:<12} {b['correct']:>6} {b['wrong']:>6} {b['failed']:>6} {b['total']:>6} {cp:>6.1f}% {wp:>6.1f}%")

    print("\n--- Pre-registered thresholds ---\n")
    for strat in ["path-range", "symbol", "content-digest", "multi"]:
        a = agg[strat]
        if a["total"] == 0:
            continue
        wp = 100 * a["wrong"] / a["total"]
        cp = 100 * a["correct"] / a["total"]
        if wp > 2:
            v = "DISQUALIFIED (wrong-resolution > 2%)"
        elif cp >= 85 and wp <= 2:
            v = "USABLE (correct >= 85%, wrong <= 2%)"
        else:
            v = "FALL BACK (does not clear both thresholds)"
        print(f"  {strat:<20} {v}")

    print("\n--- Decision procedure ---\n")
    usable = []
    disqualified = []
    for strat in ["path-range", "symbol", "content-digest", "multi"]:
        a = agg[strat]
        if a["total"] == 0:
            continue
        wp = 100 * a["wrong"] / a["total"]
        cp = 100 * a["correct"] / a["total"]
        if wp > 2:
            disqualified.append(strat)
        elif cp >= 85 and wp <= 2:
            usable.append(strat)

    if usable:
        print(f"  Usable strategies: {', '.join(usable)}")
        print("  The anchor object in schema/claim.schema.json should use")
        print(f"  the best-performing usable strategy: {usable[0] if len(usable) == 1 else 'multi (combine all usable)'}")
    elif disqualified:
        print(f"  All strategies DISQUALIFIED: {', '.join(disqualified)}")
        print("  Fall back to coarser granularity per docs/anchoring.md")
        print("  §Decision procedure: claims about module-level or")
        print("  interface-level invariants rather than spans. A smaller")
        print("  honest product beats a larger one built on anchors that lie.")
    else:
        print("  No strategy cleared both thresholds. Fall back to coarser granularity.")


def main():
    all_results = {}
    for repo_name, repo_path in REPOS:
        if not os.path.exists(repo_path):
            print(f"  {repo_name}: not found, skipping")
            continue
        results = run_simulation(repo_name, repo_path)
        all_results[repo_name] = results

    print_report(all_results)

    output_path = os.path.join(os.path.dirname(__file__), "E1-anchor-survival-result.json")
    serializable = {}
    for repo_name, results in all_results.items():
        if results is None:
            serializable[repo_name] = None
            continue
        serializable[repo_name] = {
            strat: {k: dict(v) for k, v in type_buckets.items()}
            for strat, type_buckets in results.items()
        }
    with open(output_path, "w") as f:
        json.dump(serializable, f, indent=2)
    print(f"\nRaw results saved to {output_path}")


if __name__ == "__main__":
    main()
