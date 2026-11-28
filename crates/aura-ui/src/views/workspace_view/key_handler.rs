use super::AuraWorkspaceView;
use gpui::*;

impl AuraWorkspaceView {
    pub(super) fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if (event.keystroke.modifiers.control || event.keystroke.modifiers.platform)
            && (event.keystroke.key == "s" || event.keystroke.key == "S")
        {
            if self.state.environment_editor.is_some() {
                if let Err(e) = self.state.save_environment_editor() {
                    self.state.status_message = Some(format!("Failed to save environment: {e}"));
                }
                cx.notify();
                return;
            }
            self.dispatch_save_active_tab(false, cx);
            return;
        }

        let key = event.keystroke.key.as_str();

        if self.state.environment_editor.is_some() {
            if key == "escape" {
                self.state.request_close_environment_editor();
                cx.notify();
            }
            return;
        }

        if self.state.editor_dialog.is_some() {
            if key == "escape" {
                self.state.cancel_editor_dialog();
                cx.notify();
            }
            return;
        }

        if self.state.rename_dialog.is_some() {
            if key == "escape" {
                self.state.cancel_rename();
                cx.notify();
            }
            return;
        }

        if self.state.move_to_dialog.is_some() {
            match key {
                "escape" => self.state.cancel_move_to(),
                "up" => self.state.move_to_select_prev(),
                "down" => self.state.move_to_select_next(),
                _ => return,
            }
            cx.notify();
            return;
        }

        if self.state.is_command_palette_open {
            match key {
                "escape" => self.state.close_command_palette(),
                "up" => self.state.palette_select_prev(),
                "down" => self.state.palette_select_next(),
                _ => return,
            }
            cx.notify();
            return;
        }

        if key == "escape" && self.state.active_input_target.is_some() {
            self.state.blur_active_input();
            cx.notify();
            return;
        }

        if key == "f2" {
            if let Some(selected) = self.state.workspace_store.selected_path.clone() {
                let is_folder = selected.is_dir();
                self.state.request_rename(selected, is_folder);
                cx.notify();
            }
        }
    }
}
