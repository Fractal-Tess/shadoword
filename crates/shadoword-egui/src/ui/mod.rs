pub(crate) mod constants;
pub(crate) mod helpers;
pub(crate) mod theme;
pub(crate) mod sidebar;
pub(crate) mod status_bar;
pub(crate) mod general;
pub(crate) mod settings;
pub(crate) mod models;
pub(crate) mod history;
pub(crate) mod about;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Page {
    General,
    Models,
    History,
    Settings,
    About,
}
