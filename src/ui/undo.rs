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
        self.offer_action(ToastLevel::Success, text, "Undo", true, false, restore);
    }

    /// Show `text` with a `label` button that runs `action` when clicked.
    pub(crate) fn offer_action(
        &mut self,
        level: ToastLevel,
        text: impl Into<String>,
        label: &str,
        announce: bool,
        pinned: bool,
        action: impl FnOnce(&mut App) -> Result<(), String> + 'static,
    ) {
        let id = self.toasts.push_action(level, text, label, pinned);
        self.undo.entries.push(Entry {
            id,
            action: Box::new(action),
            announce,
        });
    }

    /// Run actions for clicked buttons and forget ones whose toast has gone.
    pub(crate) fn pump_undo(&mut self) {
        for id in self.toasts.take_actions() {
            let Some(pos) = self.undo.entries.iter().position(|e| e.id == id) else {
                continue;
            };
            let entry = self.undo.entries.remove(pos);
            match (entry.action)(self) {
                Ok(()) if entry.announce => self.toasts.push(ToastLevel::Success, "Restored"),
                Ok(()) => {}
                Err(e) => self.toasts.push(ToastLevel::Warning, e),
            }
        }
        let live = self.toasts.live_actions();
        self.undo.entries.retain(|e| live.contains(&e.id));
    }
}
