# Git Conventions for Video2CRT — Commit format, scope boundary, what's owned

User-facing preferences captured 2026-09-04. Apply these on every commit + push.

## Conventional Commit Format

Every commit message uses `<type>(<scope>): <subject>` where:

| type | when | examples |
|---|---|---|
| `feat` | adds new capability / gotcha / script | `feat(skills): gotcha 18 - medium.en fallback` |
| `chore` | mechanical maintenance, redact, move files | `chore: redact Windows username from doc paths` |
| `docs` | documentation-only edits | `docs: clarify Video2CRT scope boundary` |
| `fix` | bug fix in pipeline / script behavior | `fix(install_skill): handle WinGet ffmpeg path` |
| `refactor` | restructure without behavior change | `refactor(skills): reorganize whisper section into 3 stages` |

Subject is imperative ("add", not "added"), no period at end, no capital first letter unless proper noun. Body is optional, only when the commit touches multiple subsystems.

## Author Identity

Always commit as `asaialabs <asaialabs@local>` and not `Hermes <hermes@local>`. The `Hermes` default identity comes from the desktop tool; the project wants the user as author for GitHub attribution. Use:

```bash
git -c user.name="asaialabs" -c user.email="asaialabs@local" commit -m "..."
```

If you forget this flag, the commit will go through with `Hermes <hermes@local>` and the GitHub avatar will be a generic icon instead of `asaialabs`.

## Scope Boundary — What's Video2CRT's, what isn't

Video2CRT owns (touch these freely):

- `C:\Users\asaialabs\Documents\Hermes\Video2CRT\` — project root (workspace)
- `C:\Users\asaialabs\AppData\Local\hermes\skills\video-crt-geom-libplacebo\` — the skill definition
- `C:\Users\asaialabs\AppData\Local\hermes\Video2CRT\` — HermesFullSetup mirror of project, used to bootstrap a fresh Hermes install
- GitHub remotes `wuzkkavin/Video2CRT` (public) and `wuzkkavin/HermesFullSetup` (private)

DO NOT touch (different worktree):

- `C:\Users\asaialabs\opencode\workspace\opencode-full-setup\` — OpenCode CLI's separate worktree. Hermes Desktop sidebar may list it as a project, but it is NOT Video2CRT's concern. User clarification 2026-09-04: "井水不犯河水" (don't mix Video2CRT with this OpenCode project).
- Any other local git repo under `C:\Users\asaialabs\` (there are several — `codex-full-setup`, `dashi-taskboard-zh-tw`, etc. None are Video2CRT's).

If a tool call finds itself touching `C:\Users\asaialabs\opencode\...` or any non-Video2CRT path, **stop and report to the user**. Do not assume cross-project benign behavior.

## Username Redaction in Public Commits

The repo `wuzkkavin/Video2CRT` is **public** on GitHub. Before every commit, scan staged files for:

| Pattern | Action |
|---|---|
| `C:\Users\asaialabs\...` in any .md, .py, .bat, .glsl | Replace with `C:\Users\<you>\...` (use the literal placeholder `<you>`) |
| Real email addresses (not `@users.noreply.github.com`) | Replace or remove |
| API keys / tokens (`sk-`, `ghp_`, etc.) | STOP, do not commit |
| 16-digit numbers spanning multiple ASR segments | False-positive — leave alone (they're not credit cards) |

The scanner:

```python
import os, re
patterns_to_redact = [
    (r"C:[\\/]Users[\\/]asaialabs[\\/]", "C:\\Users\\<you>\\"),
    (r"C:/Users/asaialabs/", "C:/Users/<you>/"),
]
for dirpath, _, files in os.walk(repo_root):
    for f in files:
        if f.endswith((".mp4", ".wav", ".jpg", ".png")):  # binary in .gitignore
            continue
        fp = os.path.join(dirpath, f)
        text = open(fp, encoding='utf-8', errors='ignore').read()
        for pat, repl in patterns_to_redact:
            if re.search(pat, text):
                new = re.sub(pat, repl, text)
                open(fp, 'w', encoding='utf-8').write(new)
```

This redactor was extracted from a session event (2026-09-04 zsKjUez1xdU final commit) where the user's Windows username had leaked into HANDOFF.md path examples. **Re-run before every public commit.**

## Push Strategy: Two Remotes Always Sync

Video2CRT lives in two GitHub repos:

| Remote | Visibility | Purpose |
|---|---|---|
| `wuzkkavin/Video2CRT` | public | standalone project for any agent / user to clone |
| `wuzkkavin/HermesFullSetup` | private | project mirror inside hermes-bootstrap repo; gives hermes a way to recover Video2CRT |

Workflow after each commit to `Documents/Hermes/Video2CRT/`:

```bash
# 1. Commit + push standalone (Video2CRT/)
cd "C:/Users/asaialabs/Documents/Hermes/Video2CRT"
git add <changed files>
git -c user.name="asaialabs" -c user.email="asaialabs@local" commit -m "<conventional commit msg>"
git push origin main

# 2. Mirror + push HermesFullSetup
cp <changed files> "C:/Users/asaialabs/AppData/Local/hermes/Video2CRT/"
cd "C:/Users/asaialabs/AppData/Local/hermes"
git add Video2CRT/<changed files>
git -c user.name="asaialabs" -c user.email="asaialabs@local" commit -m "..."
git push origin main
```

This needs to be two separate commits because the two repos have different commit histories. Do not attempt to make them share history (no subtree merge) — the histories drift independently and that's fine.

## Handling GitHub "Create Pull Request" Popups

After every `git push`, GitHub.com (in browser) and GitHub Desktop may show a "💡 Create Pull Request" or "Compare & pull request" button. For the two repos above (sole-owner direct-main), **clicking this button is a no-op**: GitHub auto-merges instantly because there's no reviewer conflict, no orphan PR is created, the commit is already on main.

Worth NOT clicking only if:
- The repo has GitHub Actions / CI that requires checks
- The repo is a fork intended for upstream contribution

Since Video2CRT and HermesFullSetup meet neither criterion, **dismiss or auto-merge the PR popup freely**. If you want to verify no orphan PR was left, run `gh pr list --repo wuzkkavin/Video2CRT` afterward.

(See gotcha 23 in main SKILL.md for the full rationale.)
