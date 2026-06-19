# HTML + Plain Text Licence Template Alignment Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Align licencify's built-in templates with authoritative licence text from SPDX and fix HTML template formatting based on research.

**Architecture:** Two-phase approach — (1) update all `.txt` templates to match SPDX authoritative text exactly, (2) create proper HTML templates using `<pre>` (the practical standard used by choosealicense.com/GitHub).

**Tech Stack:** Rust, Tera templates, SPDX licence-list-data

---

## Research Findings

### Plain Text Format (SPDX Authority)

- **Source:** `github.com/spdx/license-list-data/main/text/`
- Line endings: Unix LF (`0a`)
- Line wrapping: ~80 characters per line
- No trailing blank line — file ends with last character of licence text
- Placeholders: `<year>`, `<copyright holders>`, `<copyright holder>` (angle-bracket style)

### Current Divergence (our templates vs SPDX)

Our MIT template wraps at ~65-70 chars; SPDX wraps at ~80 chars. The text content is identical but line breaks differ. This is cosmetic but matters for verbatim reproduction.

### HTML Format — Three Patterns Exist

| Pattern                                | Used By                                    | Complexity                |
| -------------------------------------- | ------------------------------------------ | ------------------------- |
| `<pre>` wrapper                        | choosealicense.com (GitHub), most projects | Simple                    |
| Semantic HTML (`<div>`, `<p>`, `<ul>`) | SPDX official                              | Complex, machine-readable |
| Markdown code fence                    | README files                               | Trivial                   |

**Recommendation: `<pre>` for our HTML templates.**

- Matches GitHub/choosealicense.com standard
- Preserves exact plain text formatting
- Simple to maintain (one wrapper around plain text)
- SPDX semantic HTML is a separate machine-readable format, not what users write

### What SPDX HTML Looks Like (for reference)

```html
<div class="optional-license-text">
  <p>MIT License</p>
</div>
<div class="replaceable-license-text">
  <p>Copyright (c) &lt;year&gt; &lt;copyright holders&gt;</p>
</div>
<p>Permission is hereby granted...</p>
```

→ Uses `<var>`, `<span>` with CSS classes for machine parsing. **Not suitable** for our use case — this is SPDX's machine-readable format, not a human-facing HTML file.

### What choosealicense.com Uses (our target)

```html
<pre id="license-text">
MIT License

Copyright (c) [year] [fullname]

Permission is hereby granted, free of charge, ...
</pre>
```

→ Simple `<pre>` wrapping plain text. **This is what we should match.**

---

## Current State Analysis

### What's already correct ✓

- All 14 `.html.tera` files exist with `<pre>` / `</pre>` wrappers ✓
- `licences.rs` returns `TemplateSet { txt, html }` ✓
- `resolution.rs` selects template by format ✓
- `--format txt|html` CLI flag works ✓
- 75 tests passing ✓

### What needs fixing ✗

1. **Plain text templates don't match SPDX line wrapping** — our templates wrap at ~65 chars, SPDX at ~80
2. **HTML templates use `<pre>` with no `id` attribute** — GitHub uses `<pre id="license-text">`
3. **HTML templates have no `<head>`/charset** — bare `<pre>` without HTML document structure
4. **Line endings** — both use LF ✓ (already correct)

---

## Proposed Approach

### Phase 1: Fix Plain Text Templates to Match SPDX

Update all 13 non-proprietary templates to match the SPDX authoritative text exactly (line wrapping, wording, placeholders).

The proprietary template stays as-is since it's not an SPDX licence.

### Phase 2: Improve HTML Templates

Update all 14 `.html.tera` files to use a minimal valid HTML document wrapper:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>{{ licence_name }} License</title>
  </head>
  <body>
    <pre>
{{ licence_text }}
</pre
    >
  </body>
</html>
```

This gives users a self-contained HTML file that renders correctly in any browser.

### Phase 3: Align Tera Placeholders with SPDX

SPDX uses `<year>` and `<copyright holders>` — our templates use `{{ year }}` and `{{ author }}`. The `replace_spdx_placeholders()` function in `template.rs` already handles the post-render conversion, so we can use SPDX-style placeholders directly in templates for maximum fidelity.

**Decision needed:** Should we change templates to use SPDX `<year>` placeholders, or keep Tera `{{ year }}`? The post-render regex already converts both, so either works. Keeping `{{ year }}` is simpler for Tera.

---

## Step-by-Step Plan

### Task 1: Fetch all SPDX authoritative plain texts

**Objective:** Download all 13 SPDX licence texts (everything except proprietary) for reference.

**Files:**

- Temporary: reference texts for comparison

**Steps:**

1. `curl` each SPDX text from `raw.githubusercontent.com/spdx/license-list-data/main/text/{ID}.txt`
2. Save to a temp directory for comparison
3. Compare each against our current `.tera` template
4. Note differences (line wrapping, wording)

**Command:**

```bash
mkdir -p /tmp/spdx-ref
for id in MIT Apache-2.0 GPL-3.0-only GPL-2.0-only AGPL-3.0-only LGPL-3.0-only MPL-2.0 BSD-2-Clause BSD-3-Clause Unlicense CC0-1.0 ISC WTFPL; do
  curl -s "https://raw.githubusercontent.com/spdx/license-list-data/main/text/${id}.txt" -o "/tmp/spdx-ref/${id}.txt"
  echo "Downloaded ${id}.txt ($(wc -l < /tmp/spdx-ref/${id}.txt) lines)"
done
```

---

### Task 2: Update MIT template to match SPDX

**Objective:** Replace MIT `.tera` template content with exact SPDX text.

**Files:**

- Modify: `templates/licence/mit.tera`

**Steps:**

1. Read SPDX MIT text from `/tmp/spdx-ref/MIT.txt`
2. Replace template content, keeping `{{ year }}` and `{{ author }}` for the copyright line
3. Verify: `diff <(curl -s https://raw.githubusercontent.com/spdx/license-list-data/main/text/MIT.txt | sed 's/<year>/{{ year }}/;s/<copyright holders>/{{ author }}/') templates/licence/mit.tera`

**Commit:** `fix(templates): align MIT plain text with SPDX authoritative text`

---

### Task 3: Update Apache-2.0 template to match SPDX

**Objective:** Replace Apache-2.0 `.tera` with exact SPDX text.

**Files:**

- Modify: `templates/licence/apache-2.0.tera`

**Steps:**

1. Read SPDX Apache-2.0 text
2. Replace template content — Apache has no variable placeholders (pure SPDX text)
3. Verify alignment

**Commit:** `fix(templates): align Apache-2.0 plain text with SPDX`

---

### Task 4: Update remaining 11 templates to match SPDX

**Objective:** Align all remaining non-proprietary templates.

**Files:**

- Modify: `templates/licence/gpl-3.0-only.tera`
- Modify: `templates/licence/gpl-2.0-only.tera`
- Modify: `templates/licence/agpl-3.0-only.tera`
- Modify: `templates/licence/lgpl-3.0-only.tera`
- Modify: `templates/licence/mpl-2.0.tera`
- Modify: `templates/licence/bsd-2-clause.tera`
- Modify: `templates/licence/bsd-3-clause.tera`
- Modify: `templates/licence/unlicense.tera`
- Modify: `templates/licence/cc0-1.0.tera`
- Modify: `templates/licence/isc.tera`
- Modify: `templates/licence/wtfpl.tera`

**Steps:**
For each template:

1. Read SPDX text from `/tmp/spdx-ref/{ID}.txt`
2. Replace template content, converting SPDX `<year>`/`<copyright holders>` to `{{ year }}`/`{{ author }}` where applicable
3. Verify alignment

**Commit:** `fix(templates): align remaining plain text templates with SPDX`

---

### Task 5: Rebuild HTML templates from corrected plain text

**Objective:** Recreate all `.html.tera` files from the now-SPDX-aligned plain text templates.

**Files:**

- Recreate: all 14 `templates/licence/*.html.tera` files

**Template pattern:**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>License</title>
  </head>
  <body>
    <pre>
{{ licence_text }}
</pre
    >
  </body>
</html>
```

**Implementation approach:**
The HTML templates should be standalone — not wrapping the plain text at runtime. Instead, they should contain the full HTML document with the licence text inline. This means:

1. Each `.html.tera` is a self-contained HTML document
2. Placeholders (`{{ year }}`, `{{ author }}`, `{{ company }}`, etc.) are embedded directly
3. No runtime wrapping needed — the template IS the HTML

**Example `mit.html.tera`:**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>MIT License</title>
  </head>
  <body>
    <pre>
MIT License

Copyright (c) {{ year }} {{ author }}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
</pre
    >
  </body>
</html>
```

**For proprietary.html.tera:**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>Proprietary License</title>
  </head>
  <body>
    <pre>
Copyright (C) {{ company }} - All Rights Reserved

Unauthorized copying of this file, via any medium is strictly prohibited
Proprietary and confidential
Written by {{ author }} &lt;{{ email }}&gt;, {{ date }}
</pre
    >
  </body>
</html>
```

Note: `&lt;` and `&gt;` for angle brackets inside HTML `<pre>`.

**Steps:**

1. Read each corrected `.tera` plain text template
2. Generate corresponding `.html.tera` with HTML document wrapper
3. Verify each renders correctly: `cargo test`
4. Spot-check a few by opening `.html` files in browser

**Commit:** `fix(templates): recreate HTML templates as self-contained HTML documents`

---

### Task 6: Update tests for new HTML format

**Objective:** Ensure tests reflect the new HTML document structure.

**Files:**

- Modify: `src/template.rs` (tests)

**Steps:**

1. Update `render_mit_template_html` test to check for `<!DOCTYPE html>` and `<pre>` and `</pre>`
2. Update `render_proprietary_template_html` test similarly
3. Add test for `render_apache_template_html`
4. Run `cargo test` — all pass

**Commit:** `test: update HTML template tests for self-contained HTML format`

---

### Task 7: Verify end-to-end

**Objective:** Manual smoke test of both formats.

**Steps:**

1. `cargo run -- add mit --author "Test User" --format txt -Y` → check LICENCE.txt
2. `cargo run -- add mit --author "Test User" --format html -Y` → check LICENCE.html
3. Open LICENCE.html in browser — verify it renders as a formatted licence page
4. `cargo run -- add proprietary --author "Test" --company "Acme" --email "test@acme.com" --format html -Y` → verify HTML

---

## Files Likely to Change

| File                                                                                             | Action                                       |
| ------------------------------------------------------------------------------------------------ | -------------------------------------------- |
| `templates/licence/*.tera` (13)                                                                  | Replace content with SPDX-authoritative text |
| `templates/licence/proprietary.tera`                                                             | No change (not SPDX)                         |
| `templates/licence/*.html.tera` (14)                                                             | Recreate as self-contained HTML documents    |
| `src/template.rs`                                                                                | Update HTML-related tests                    |
| No changes to: `licences.rs`, `resolution.rs`, `cli.rs`, `commands/add.rs`, `commands/update.rs` | Architecture already correct                 |

---

## Risks & Tradeoffs

| Risk                                       | Mitigation                                                                                                    |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| SPDX text changes between versions         | Pin to a specific SPDX release tag, not `main` branch                                                         |
| Line wrapping differences                  | Match SPDX exactly; diff to verify                                                                            |
| HTML `<pre>` doesn't handle special chars  | For non-proprietary SPDX licences, no special chars needed. Proprietary uses `&lt;`/`&gt;` for angle brackets |
| Tera `{{ }}` vs SPDX `<year>` placeholders | Keep `{{ }}` style — `replace_spdx_placeholders()` handles both post-render                                   |

---

## Open Questions

1. **Should HTML templates include `<title>` with the licence name?** → Yes, for browser tab display
2. **Should we pin SPDX source to a release tag?** → Recommend yes (e.g., `license-list-data/3.24.0`) but can do in a follow-up
3. **Proprietary HTML** — should it also be a full HTML document? → Yes, consistency
