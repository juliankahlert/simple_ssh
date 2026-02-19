# Commit Message Style Guide

This document defines the commit message style for the simple_ssh project, combining principles from [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/), the [Linux Kernel](https://www.kernel.org/doc/html/v4.10/process/submitting-patches.html), and [Embedded Artistry](https://embeddedartistry.com/fieldatlas/source-control-commit-guidelines/) guidelines with project-specific styling rules.

---

## Language

- Use English
- Use Simple present
- Use simple language

## Creating Commits

All commits **must** be signed off using the `-s` flag:

```bash
git commit -s -m "<MESSAGE>"
```

The `-s` flag adds a `Signed-off-by` line to the commit message footer, certifying that you have the right to submit the code under the project's license (Developer Certificate of Origin).

### Examples

```bash
# Simple commit
git commit -s -m "src: Add password authentication support"

# Commit with body (using editor)
git commit -s

# Amend last commit (also re-sign)
git commit -s --amend
```

---

## Format

```
<scope>: <subject>

<body>

<footer>
```


---

## Character Set

### ASCII Only

Commit messages **must** use **ASCII characters only**.

```
✅ src: lib: Add IPv6 link-local address support
✅ docs: Update README with installation instructions

❌ src: Add niño support                   # Non-ASCII character (ñ)
❌ docs: Add 日本語 translation             # Non-ASCII characters
❌ feat: Add emojis 🎉                     # Emoji characters
```

### Markdown Allowed

[Markdown](https://commonmark.org/help/) formatting is permitted and encouraged in the body and footer sections:

```
✅ docs: Add troubleshooting guide

Use `code` formatting for inline commands:
- `cargo build`
- `npm run dev`

**Note:** This requires Node.js 18+.

See [Conventional Commits](https://conventionalcommits.org) for more info.
```

### Exceptions

The following are **not** considered violations:
- URLs containing non-ASCII characters (e.g., `https://example.com/über`)
- Email addresses in `Signed-off-by` or `Co-authored-by` lines
- Markdown code blocks containing example code with non-ASCII strings

---

## Scope Rules

### 1. Base Scope Required

**Always include the base scope.** Never omit the root context.

```
✅ src: Add session builder
✅ examples: Create basic SSH example
✅ pages: Initialize Vue project
✅ .github: Update deploy workflow

❌ Add session builder              # Missing base scope
❌ Create basic SSH example         # Missing base scope
```

### 2. Omit Leading Dots

Remove leading dots from scope names.

```
✅ github: Update deploy workflow    # Not .github:
✅ gitignore: Add node_modules      # Not .gitignore:
✅ env: Add environment variables   # Not .env:
```

### 3. Keep Scope Concise

If multiple sub-scopes change, use the common parent scope.

```
# When modifying both:
# - src/session/builder.rs
# - src/session/connector.rs

✅ src: Update session builder and connector

❌ src: session: builder: Update builder
❌ src: session: connector: Update connector
```

### 4. Smart Middle Scope Omission

For deep nesting, omit middle scopes that don't add clarity, but **only if** removing them doesn't create ambiguity.

```
# Original long scope:
src: components: editor: codearea: highlighting:

# Can become (highlighting is unambiguous in src):
✅ src: highlighting: Add Rust syntax highlighting

# But keep middle scope if ambiguous:
# src: components: icons: rust: vs src: components: icons: terminal:
✅ src: icons: rust: Adjust icon size      # Keep 'rust' to distinguish
✅ src: icons: terminal: Fix alignment     # Keep 'terminal' to distinguish
```

### 5. Capitalize Subject

After the final scope (and colon), start the subject with a capital letter.

```
✅ src: Add session builder
✅ examples:basic: Implement CLI parsing
✅ pages:components: Create VSCode editor

❌ src: add session builder              # Lowercase subject
❌ examples:basic: implement CLI parsing # Lowercase subject
```

---

## Subject Rules

1. **Imperative mood**: Use "Add" not "Added" or "Adds"
2. **No period at end**
3. **Max 72 characters** for the entire first line
4. **Describe what and why**, not how

```
✅ src: Add password authentication support
✅ examples: Refactor to use shared CLI module

❌ src: Added password authentication support.  # Past tense + period
❌ src: This commit adds password auth support  # Doesn't start with verb
❌ src: Add password authentication support.    # Trailing period
```

---

## Body Rules

The body is **never optional**. Every commit must include a body that explains the change.

The body should NOT:

1. **Use bullet point lists**

The body should:

1. **Use 1-3 proper paragraphs**
2. **Explain the motivation** for the change
3. **Contrast with previous behavior** (for fixes)
4. **Reference related issues** when applicable
5. **Wrap at 72 characters**
6. **Separate from subject with blank line**

### Avoid Bullet Point Lists

The commit body should flow as narrative prose, not as enumerated items. Bullet points make commits harder to read and often indicate the commit is doing too much.

```
❌ src: Add user authentication

- Add password field to login form
- Add validation for email format
- Add API endpoint for login
- Add session management
```

```
✅ src: Add user authentication

Add complete email/password authentication to the application.
Users can now log in with their credentials, which are validated server-side
and generate a secure session token. The implementation follows OAuth 2.0
best practices for password flow.

The login form now includes real-time email validation and displays helpful
error messages for invalid credentials. Session tokens are stored securely
with appropriate expiration and refresh mechanisms.
```

### Never Omit the Body

```
✅ src: lib: Add IPv6 link-local address support

Enable link-local address connections with fe80::/10 prefix,
required for devices like Raspberry Pi on the local network.

❌ src: lib: Add IPv6 link-local address support    # Missing body
```

---

## Footer Rules

Use the footer for:

1. **Breaking changes**: Start with `BREAKING CHANGE:`
2. **Issue references**: `Closes #123`, `Fixes #456`, `Refs #789`
3. **Co-authors**: `Co-authored-by: Name <email>`

```
src: lib: Remove deprecated Session::new()

The old Session::new() method has been deprecated since v0.2.0. Replace
with Session::init() which provides the builder pattern.

BREAKING CHANGE: Session::new() has been removed. Use Session::init()
followed by builder methods instead.

Closes #55
```

---

## Examples by Context

### Source Code Changes

```
src: lib: Add PTY auto-resize support

src: Handle connection timeout gracefully

src: session: Extract authentication logic

src: Reduce allocations in command execution

src: Add integration tests for SCP operations
```

### Example Changes

```
examples: Add IPv6 connection example

examples: Use shared CLI module in all examples

examples: cli: Update prompt messages
```

### Documentation Changes

```
docs: Update README with installation instructions

src: Document SessionBuilder methods

examples: Add comments to basic.rs
```

### Configuration/Build Changes

```
cargo: Add example binaries configuration

github: Update deploy workflow for Vue.js build

vite: Configure base path for GitHub Pages

gitignore: Add node_modules and dist to pages ignore

release: ci: cargo: Prepare v0.1.3 release

meta: cargo: examples: Move example binaries to examples directory
```

---

## Multi-Scope Format

When a change affects multiple areas, chain scopes with colons. List scopes from broadest to most specific.

```
✅ release: ci: cargo: Prepare v0.1.3 release
✅ meta: cargo: examples: Move example binaries to examples directory
✅ github: ci: Update deploy workflow for Vue.js build
```

---

## Historical Context

The rules have evolved. Older commits (pre-2024) may not follow all guidelines:
- Early commits may lack body text
- Some used `.github:` instead of `github:`

Current enforcement requires all guidelines be followed for new commits.

---

## Common Mistakes to Avoid

```
❌ feat: Add new feature                     # Missing scope
❌ src:add session builder                   # Space after colon
❌ src: add session builder                  # Lowercase subject
❌ src: Add session builder.                 # Trailing period
❌ .github: Update workflow                  # Leading dot in scope
❌ src:session:builder: Connector: Fix bug   # Second capital (wrong)
✅ src: builder: Fix connector bug           # Correct
```
