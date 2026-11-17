//! Toast actions: a toast with a button that runs a closure while it is on screen.
//! Used for Undo after removals in the editors, and for links such as "update available".

use crate::app::App;
use crate::ui::kit::ToastLevel;

type Action = Box<dyn FnOnce(&mut App) -> Result<(), String>>;

struct Entry {
    id: u64,
    action: Action,
    /// Confirm with a "Restored" toast when the action succeeds (Undo only).
    announce: bool,
}

#[derive(Default)]
pub struct UndoStack {
    entries: Vec<Entry>,
}

impl App {
    /// Show `text` with an Undo button that runs `restore` when clicked.
    pub(crate) fn offer_undo(
        &mut self,
        text: impl Into<String>,
        restore: impl FnOnce(&mut App) -> Result<(), String> + 'static,
    ) {
        self.offer_action(ToastLevel::Success, text, "Undo", true, restore);
    }

    /// Show `text` with a `label` button that runs `action` when clicked.
    pub(crate) fn offer_action(
        &mut self,
        level: ToastLevel,
        text: impl Into<String>,
        label: &str,
        announce: bool,
        action: impl FnOnce(&mut App) -> Result<(), String> + 'static,
    ) {
        let id = self.toasts.push_action(level, text, label);
        self.undo.entries.push(Entry {
            id,
            action: Box::new(action),
            announce,
        });
    }

    /// Run actions for clicked buttons and forget ones whose toast has gone.
    pub(crate) fn pump_undo(&mu