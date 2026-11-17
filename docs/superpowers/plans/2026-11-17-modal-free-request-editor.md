# Modal-Free Request Editor & gpui-kit Component Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the request editor into a modal-free, high-productivity interface using `gpui-kit` components—laying out authentication inputs directly upon preset selection, providing a full-height inline body editor, seamless key-value grid editing for params and headers, script sub-tabs for pre/post-scripts, and a complete test editor with quick snippets and instant test execution.

**Architecture:** 
Decompose input management into fine-grained in-place focus targets (`ActiveInputTarget`) within `AuraAppState`, eliminating `EditorDialogState` modal overlays. Integrate `gpui-kit` UI components (`Input`, `Editor`, `Button`, `TabBar`) across `request_editor_view.rs` to render direct form fields for Auth, full-height multiline editors for Body, Scripts, and Tests, and spreadsheet-style inline grid rows for Params/Headers. Connect test script evaluation directly to `AuraScriptRuntime` on both request completion and manual '▶ Run Tests' trigger.

**Tech Stack:** Rust 2021, GPUI 0.2.2, `gpui-kit = "0.7.3"`, Tokio 1.x, `deno_core` / `aura-scripting`.

**Spec:** User requirements from `/grill-me` session:
1. **Authentication**: Immediate input fields upon preset selection; no edit buttons; maximize available screen space; live JWT and Base64 previews.
2. **Body**: Full-height monospace editor directly displayed; no edit button or modal; top action bar for Format JSON, Clear, and payload size.
3. **Params & Headers**: Seamless key-value grid; clicking '+ Add' immediately inserts a new editable row; Tab navigates Key -> Value -> Next row; no modal popups.
4. **Scripts**: Sub-tabs for Pre-request Script and Post-response Script; full-height multiline editors; quick snippets.
5. **Tests**: Full-height test editor; snippet templates (`Status 200`, `JSON Field Check`, `Response Time`); auto-run after request completion + manual '▶ Run Tests' evaluator button.
6. **Component Suite**: Use `gpui-kit` components (`Input`, `Editor`, `Button`, `TabBar`).

---

## Global Constraints
- Strictly adhere to `DESIGN.md` dark palette: Canvas `#1d1d1f`, Surface Tile 1 `#272729`, Surface Tile 2 `#2a2a2c`, Borders `#333333`, Accent Blue `#0066cc`.
- Modularity rule: Keep views within ~300-400 LOC where possible by separating panel renderers.
- Asynchronous offloading: Script execution must run cooperatively and non-blocking.
- All existing tests in `aura-ui` must pass without regression.

---

## Review Focus
1. **Focus Trapping & Escape**: Escape key must blur any active input cleanly without losing data.
2. **Table Grid Tab-to-Add**: Pressing Tab on the last row's Value cell or clicking '+ Add' must append an empty row and immediately focus its Key without UI stutter.
3. **Multiline Code Editors**: Pressing Enter inside Body/Scripts/Tests must insert a newline `\n`; pressing Tab must insert 2 spaces (`  `).
4. **Auth State Synchronization**: Changing Auth preset values must immediately update the active tab's `AuraRequest.auth` and mark the tab dirty.
5. **Test Re-run Safety**: Clicking '▶ Run Tests' when no response exists or when script syntax is invalid must gracefully report the error rather than panic.

---

## Task Decomposition

### Task 1: Domain State & Active Input Target System
**Files:**
- Modify: `crates/aura-ui/src/app_state.rs`
- Test: `crates/aura-ui/tests/test_inline_input_state.rs`

- [ ] **Step 1: Write failing unit test for `ActiveInputTarget` and inline editing state**
  Verify that `AuraAppState` can focus specific cells (ParamKey, ParamValue, AuthToken, BodyText, etc.), append chars, handle backspace, handle Tab navigation, and mark tabs dirty.
- [ ] **Step 2: Define `ActiveInputTarget` enum and `ScriptSubTab` enum in `app_state.rs`**
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ActiveInputTarget {
      ParamKey(usize),
      ParamValue(usize),
      HeaderKey(usize),
      HeaderValue(usize),
      FormParamKey(usize),
      FormParamValue(usize),
      AuthBearerToken,
      AuthBasicUsername,
      AuthBasicPassword,
      AuthApiKeyName,
      AuthApiKeyValue,
      AuthAwsField(usize),
      AuthOAuth2Field(usize),
      BodyText,
      PreRequestScript,
      PostResponseScript,
      TestScript,
  }
  ```
- [ ] **Step 3: Implement inline keyboard handlers on `AuraAppState`**
  Add `set_active_input(target)`, `blur_active_input()`, `type_char_into_active(char)`, `backspace_active()`, `tab_next_input(shift: bool)`, and `enter_active_input()`.
- [ ] **Step 4: Implement '+ Add' direct row creation without modals**
  Update `add_param`, `add_header`, and `add_form_urlencoded_param` to immediately push a new `ParamItem` and focus its Key.
- [ ] **Step 5: Run tests and verify Task 1 passes**
  `cargo test -p aura-ui --test test_inline_input_state`

---

### Task 2: Seamless Modal-Free Key-Value Grids (Params & Headers) with gpui-kit
**Files:**
- Modify: `crates/aura-ui/src/views/request_editor_view.rs`
- Modify: `crates/aura-ui/src/views/workspace_view.rs`
- Test: `crates/aura-ui/tests/test_request_editor_view.rs`

- [ ] **Step 1: Write failing test verifying direct row insertion and focus for Params and Headers**
  Verify that clicking '+ Add Parameter' / '+ Add Header' appends a row with immediate inline edit capability.
- [ ] **Step 2: Refactor `render_params_panel` and `render_headers_panel` in `request_editor_view.rs`**
  - Render an interactive spreadsheet-style grid using `gpui-kit` inputs or styled inline cells with clear focus rings.
  - When focused, render cursor and highlight active cell border in Action Blue (`#0066cc`).
  - Clicking any Key or Value cell sets `ActiveInputTarget::ParamKey/ParamValue(idx)`.
  - Tab jumps Key -> Value -> Next Row.
  - Remove '+ Add First Parameter' modal calls and '✎ Edit' button modals.
- [ ] **Step 3: Wire key event routing in `workspace_view.rs`**
  Forward keystrokes when `active_input_target` is present directly to `this.state.type_char_into_active()`.
- [ ] **Step 4: Run tests and verify Task 2 passes**
  `cargo test -p aura-ui --test test_request_editor_view`

---

### Task 3: Direct Authentication Panel with Live Previews & gpui-kit
**Files:**
- Modify: `crates/aura-ui/src/views/request_editor_view.rs`
- Test: `crates/aura-ui/tests/test_auth_panel_inline.rs`

- [ ] **Step 1: Write failing test verifying Auth inputs are immediately available upon preset selection**
  Verify that selecting Bearer, Basic, API Key, AWS SigV4, or OAuth2 immediately exposes editable fields without opening a dialog.
- [ ] **Step 2: Refactor `render_auth_panel` in `request_editor_view.rs`**
  - Render full-width labeled `gpui-kit` Input fields for:
    - **Bearer Token**: Full-width token input, with real-time JWT Inspector card below.
    - **Basic Auth**: Side-by-side or stacked Username and Password inputs, with real-time Base64 preview banner below.
    - **API Key**: Key Name input, Key Value input, and Header vs Query toggle pill directly inline.
    - **AWS SigV4 & OAuth 2.0**: Grid of inputs laid out with ample space.
  - Remove all '✎ Edit ...' buttons and `OpenEditAuth` modal triggers.
- [ ] **Step 3: Run tests and verify Task 3 passes**
  `cargo test -p aura-ui --test test_auth_panel_inline`

---

### Task 4: Full-Height Body Editor with Action Bar
**Files:**
- Modify: `crates/aura-ui/src/views/request_editor_view.rs`
- Test: `crates/aura-ui/tests/test_body_panel_inline.rs`

- [ ] **Step 1: Write failing test verifying Body panel has full-height direct editing and format actions**
- [ ] **Step 2: Refactor `render_body_panel` in `request_editor_view.rs`**
  - Remove '✎ Edit Body' button and modal dialog.
  - Render a top action toolbar: '⚡ Format JSON', 'Clear', and payload byte size indicator.
  - Render a full-height multiline editor area (`gpui_kit::component::input::Editor` or styled monospace textarea) filling all available vertical space.
  - Support Enter (`\n`) and Tab (`  `) indentation.
- [ ] **Step 3: Run tests and verify Task 4 passes**
  `cargo test -p aura-ui --test test_body_panel_inline`

---

### Task 5: Scripts Panel Sub-Tabs & Tests Suite Completion
**Files:**
- Modify: `crates/aura-ui/src/app_state.rs`
- Modify: `crates/aura-ui/src/views/request_editor_view.rs`
- Modify: `crates/aura-ui/src/views/workspace_view.rs`
- Test: `crates/aura-ui/tests/test_scripts_and_tests_view.rs`

- [ ] **Step 1: Write failing test for Script sub-tabs, quick snippets, and test execution**
  Verify switching between Pre-request and Post-response sub-tabs, inserting snippets, and evaluating tests against an `AuraResponse`.
- [ ] **Step 2: Implement Scripts panel sub-tabs**
  - Add sub-tab header in `render_scripts_panel`: `[ Pre-request Script ]` and `[ Post-response Script ]`.
  - Maximize vertical screen real estate for each script with full-height multiline code editors.
  - Add quick snippet buttons: `Set Env Var`, `Get Env Var`, `Timestamp`, `Log Info`.
- [ ] **Step 3: Complete Tests panel with quick snippets and '▶ Run Tests' trigger**
  - Render full-height test code editor.
  - Add snippet insertion bar: `Status 200`, `JSON Field Check`, `Response Time < 500ms`, `Header Check`.
  - Add '▶ Run Tests' button in the toolbar that immediately evaluates tests via `AuraScriptRuntime` on the active response.
  - Display test pass/fail badge (e.g. `✓ 3/3 Passed` or `✕ 1/3 Failed`).
- [ ] **Step 4: Clean up obsolete `EditorDialogState` dialogs from `workspace_view.rs`**
  Remove modal overlay card rendering for request editor actions, simplifying `workspace_view.rs`.
- [ ] **Step 5: Run tests and verify Task 5 passes**
  `cargo test -p aura-ui --test test_scripts_and_tests_view`

---

### Task 6: Full Integration & Regression Verification
**Files:**
- Run full test suite across workspace
- Check code formatting & clippy

- [ ] **Step 1: Run all unit and integration tests**
  `cargo test --workspace`
- [ ] **Step 2: Run clippy and format check**
  `cargo clippy -p aura-ui`
- [ ] **Step 3: Verify clean build and no UI regressions**
