use gpui::App;

use crate::theme::Theme;

pub trait ThemeExt {
    /// Changes the theme.
    fn set_theme<T: AsRef<Theme>>(&mut self, theme: T);

    /// Gets an immutable reference to the theme.
    fn get_theme(&self) -> &Theme;
}

impl ThemeExt for App {
    fn set_theme<T: AsRef<Theme>>(&mut self, theme: T) {
        self.set_global::<Theme>(theme.as_ref().clone())
    }

    fn get_theme(&self) -> &Theme {
        self.global()
    }
}
