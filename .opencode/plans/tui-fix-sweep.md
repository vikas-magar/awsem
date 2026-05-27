# TUI Fix Sweep — Implementation Plan

## Phase 1: Critical Bug Fixes

### 1.1 Cursor bounds wrap-to-MAX

**Files:** `app.rs`, `event_loop.rs`

**`app.rs:152-155`** — `refresh_all()` clamps cursor/scroll to `len.saturating_sub(1)`. When `len=0`, `saturating_sub(1)` wraps to `usize::MAX`. Fix:

```rust
// Replace:
let len = self.panel_len(p);
if self.panel_cursors[p] >= len.saturating_sub(1) { self.panel_cursors[p] = len.saturating_sub(1); }
if self.panel_scrolls[p] >= len { self.panel_scrolls[p] = len.saturating_sub(1).saturating_sub(1); }

// With:
let len = self.panel_len(p);
if len > 0 {
    if self.panel_cursors[p] >= len { self.panel_cursors[p] = len - 1; }
    if self.panel_scrolls[p] >= len { self.panel_scrolls[p] = len.saturating_sub(1); }
} else {
    self.panel_cursors[p] = 0;
    self.panel_scrolls[p] = 0;
}
```

**`event_loop.rs:47`** — Same pattern in `KeyCode::Down` guard. Currently: `if c < m.saturating_sub(1)`. When `m=0`, this is `c < usize::MAX` which is always true. Change to `if m > 0 && c < m - 1`.

Also check `event_loop.rs:104` for browser Down key — same pattern.

### 1.2 Upload spinner animation

**File:** `upload.rs` render function

The `state.spinner` field is declared and used to index into the `SPINNER` character sequence, but it's never incremented. Fix: increment `state.spinner` each render frame.

This runs in the render path which is called every event loop iteration. Add `state.spinner += 1` at the start of the `render()` function (or call `app.upload.spinner += 1` in `ui.rs` before calling `crate::upload::render()`).

### 1.3 Render the invisible Logs panel

**Files:** `ui.rs`, `sidebar.rs`

**`ui.rs` — `render_dashboard()`:**
Currently renders panels 0-4 (S3, EMR, Cognito, Secrets, Lambda). Add panel 5 (logs) rendering. The logs data is fetched in `refresh_all()` but never displayed.

The layout needs adjustment. Current layout is 3 rows (2-column top, 2-column middle, full-width bottom). To add a 6th panel, change to a 3x2 grid (3 rows × 2 columns) with equal height rows. Or keep it 4 rows with a shorter row for logs.

Simplest approach: change the bottom row to a 50/50 split with Lambda on the left and Logs on the right. This keeps the 3-row layout.

The logs panel would show: timestamp, level (color-coded), message — using `table::render()`.

**`sidebar.rs`:**
Add `[6] Logs` entry after `[5] Lambda` to match the keyboard shortcut `6`.

---

## Phase 2: Missing Features

### 2.1 Detail popup (Enter on any item)

**Files:** `event_loop.rs`, `app.rs`

Currently only `Enter` on S3 panel works (opens BucketBrowser). Extend to panels 1-4:

- **Panel 1 (EMR):** `Enter` → fetch `emr_jobs()` for the selected VC → show jobs in detail popup or a new view mode
- **Panel 2 (Cognito):** `Enter` → show user attributes, groups, timestamps in detail popup
- **Panel 3 (Secrets):** `Enter` → show secret metadata (ARN, description, last_changed, rotation status) in detail popup
- **Panel 4 (Lambda):** `Enter` → show function config (handler, memory, timeout, last_modified, runtime) in detail popup

Each popup sets `app.popup_title` and `app.popup_lines`, then `app.mode = ViewMode::DetailPopup`.

### 2.2 EMR job browser

**File:** `event_loop.rs`

The `emr_jobs()` fetcher exists but is never wired. When user presses `Enter` on an EMR VC, fetch jobs and either:
- Show in the detail popup (simple), or
- Enter a new `ViewMode::EmrJobs` mode with a table (complex)

Start with the detail popup approach. Later can add a full job browser mode.

### 2.3 Confirmation dialogs

**File:** `event_loop.rs`, `app.rs`

Add a `pending_confirm: Option<(String, String, Box<dyn FnOnce(&mut App)>)>` field to `App`. When a destructive action is triggered, instead of executing immediately, set `pending_confirm` with title + message + callback. Show a confirmation popup. On Enter, execute the callback. On Esc, clear.

But `FnOnce` is hard with `App` and borrow checking. Simpler approach: use a string-based state machine.

```rust
// App fields:
pub confirm_title: String,
pub confirm_msg: String,
pub confirm_action: Option<ConfirmAction>,

// In event_loop:
if let Some(action) = app.confirm_action.take() {
    match key.code {
        KeyCode::Enter => { app.execute_confirm(action); }
        KeyCode::Esc => {}
        _ => { app.confirm_action = Some(action); }
    }
    continue;
}
```

Where `ConfirmAction` is an enum: `DeleteBucket(usize)`, `DeleteUser(usize)`, `DeleteSecret(usize)`, `DeleteFunction(usize)`, `DeleteVc(usize)`, etc.

### 2.4 Update help screen

**File:** `help.rs`

Add missing keybindings to the help overlay:
- `Space` — Select file/dir for upload (upload mode)
- `d` — Download selected S3 object (browser mode)
- `D` — Download all objects in prefix (browser mode)
- `BackTab` — Previous form field (form mode)
- `f` — Open log filter (dashboard panel 6)
- `6` — Jump to Logs panel
- `Esc` — Close panel / go back (context-dependent)

---

## Phase 3: Polish

### 3.1 S3 sort toggle

**File:** `event_loop.rs`

Add a keybinding (e.g. `s` in BucketBrowser mode) that toggles `browser_sort_desc` and re-sorts `browser_items` by name. Currently the sort indicator renders but no key triggers it.

```rust
// In BucketBrowser match:
KeyCode::Char('s') => {
    app.browser_sort_desc = !app.browser_sort_desc;
    app.browser_items.sort_by(|a, b| {
        let cmp = a.key.cmp(&b.key);
        if app.browser_sort_desc { cmp.reverse() } else { cmp }
    });
}
```

### 3.2 S3Upload dead code cleanup

**Files:** `form.rs`, `app.rs`

`FormAction::S3Upload` was replaced by UploadMode. Remove it from:
- `form.rs:27` — enum definition
- `form.rs:59-61` — form field definitions
- `app.rs:170` — unreachable match arm

Also remove the unused `upload_object` function from `actions.rs` if it's not used elsewhere.

### 3.3 Status bar hint fixes

**File:** `ui.rs`

The status bar at the bottom shows keyboard hints. Sync them with actual keybindings. The current hint hardcodes text — should reflect the current mode's available keys.

### 3.4 Line limit compliance

Split files exceeding 150 lines:

- **`app.rs` (175):** Extract `start_upload` and `start_download` into a new `transfer.rs` file (the shared logic for background tasks with progress tracking).
- **`upload.rs` (161):** Extract the `UploadTransfer` struct and `collect_files` helper into `transfer.rs`.
- **`event_loop.rs` (167):** Extract mode-specific key handlers into separate files: `handlers/dashboard.rs`, `handlers/browser.rs`, `handlers/upload.rs`.
