use crate::i18n::Language;
use crate::model::{
    App, BACKGROUNDS, CATEGORIES, INPUT_NAME_LIMIT, InputMode, Inventory, Preferences,
    clean_input_name,
};
use crate::weather::{self, Unit};
use serde::Serialize;
use std::time::{Duration, Instant};

pub const WEATHER_SECTION: usize = 2;
pub const INPUTS_SECTION: usize = 3;
pub const SYSTEM_SECTION: usize = 4;
pub const BRIDGE_SECTION: usize = 5;
pub const SECTION_COUNT: usize = 6;
pub const APPS_COLUMNS: usize = 6;
pub const HOME_STRIP_LEFT: f32 = 84.;
// Six favourites in full and the seventh cut off and fading out at the right (Apple TV rows):
// the strip ends after the sixth tile, so a focused tile never sits in the fade.
pub const HOME_STRIP_RIGHT: f32 = 1748.;
pub const HOME_TILE_WIDTH: f32 = 256.;
pub const HOME_TILE_PITCH: f32 = 281.6;
pub const HOME_FADE_LEFT: f32 = 8.;
pub const HOME_FADE_RIGHT: f32 = 1910.;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Page {
    Home,
    Apps,
    Dom,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Target {
    Nav(usize),
    TvSettings,
    Recent(String),
    RecentEmpty,
    RecentRetry,
    App(String),
    Input(String),
    Browse,
    Filter(usize),
    ClearFilter,
    Search,
    ClearSearch,
    Sidebar(usize),
    Setting(usize),
    GreetingName,
    GreetingNameClear,
    AppNamesHome,
    AppNamesApps,
    ClockFormat,
    ClockTimeFormat,
    Background(usize),
    InputMode(String),
    InputRename,
    InputNameReset,
    Choice(usize),
    CloseDialog,
    Retry,
    DismissNotice,
    Option(usize),
    Key(usize),
    WeatherConfigure,
    WeatherToggle,
    WeatherLocation,
    WeatherTemperature,
    WeatherQuery,
    WeatherSearch,
    WeatherClear,
    WeatherResult(usize),
    HaTile(String),
    HaAddress,
    HaCode,
    HaPair,
    HaPairCancel,
    HaToggle,
    LockPin,
    LockRequest,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    pub fn center(self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn inset(self, n: f32) -> Self {
        Self::new(self.x + n, self.y + n, self.w - 2. * n, self.h - 2. * n)
    }
}

#[derive(Clone, Debug)]
pub struct Control {
    pub target: Target,
    pub rect: Rect,
    pub scrollable: bool,
}

#[derive(Clone, Debug)]
pub enum Overlay {
    Options { id: String, restore: Target },
    Shortcut { id: String, restore: Target },
    Recents { restore: Target },
    Keyboard,
    SystemKeyboard,
    Camera { id: String },
    Lock,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Subpage {
    AppNames,
    Greeting,
    Clock,
    ClockFormat,
    TimeFormat,
    Input(String),
    WeatherLocation,
    Temperature,
    Language,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum TextField {
    Apps,
    Location,
    Name,
    InputName,
    HaUrl,
    HaCode,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Save,
    /// Read the app inventory again, keeping decoded icons of unchanged apps.
    Refresh,
    /// Refresh apps: read the inventory and every app icon again.
    Reload,
    Launch(String),
    Recent {
        generation: u64,
        command: crate::recents::Command,
    },
    Quit,
    Weather(weather::Command),
    BridgeRefresh,
    BridgePress(String),
    StartPairing,
    StopPairing,
    Watch(Option<String>),
    LockUnlock(String),
    LockRequest,
}

pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Space",
    "Delete", "Clear", "Done",
];

pub struct State {
    pub page: Page,
    pub focus: Target,
    pub inventory: Inventory,
    pub preferences: Preferences,
    pub query: String,
    pub filter: usize,
    pub section: usize,
    pub subpage: Option<Subpage>,
    pub scroll: f32,
    pub home_scroll: f32,
    pub overlay: Option<Overlay>,
    pub moving: Option<String>,
    pub loading: bool,
    /// Set by Refresh apps: the next inventory also reloads every app icon.
    pub reload_artwork: bool,
    pub launching: bool,
    pub notice: Option<(String, Instant)>,
    pub error: Option<String>,
    pub dirty: bool,
    pub pointer: bool,
    pub pressed: Option<Target>,
    pub previous_background: Option<(String, Instant)>,
    pub panel_started: Option<Instant>,
    pub weather: weather::Weather,
    pub keyboard_field: TextField,
    pub greeting_name_draft: String,
    pub ha_draft: String,
    pub ha_compose: String,
    pub tiles: Vec<crate::bridge::Tile>,
    pub shots: std::collections::BTreeMap<String, String>,
    pub shot_errors: std::collections::BTreeMap<String, String>,
    pub ring_id: u64,
    pub pending_watch: Option<String>,
    pub bridge_error: Option<String>,
    /// Pressed tile, the state we already showed, and when. A slow reply must not paint the old state back.
    pub ha_hold: Option<(String, String, Instant)>,
    pub pairing: Option<crate::pair::Offer>,
    pub input_name: String,
    pub recents: crate::recents::Recents,
    pub lock: crate::bridge::LockState,
    pub lock_pin: String,
    pub lock_status: String,
    pub lock_status_at: Option<Instant>,
    pub lock_since: Option<Instant>,
    /// Which play message the lock screen starts with. Picked at random each time it opens.
    pub lock_play_offset: usize,
}

pub const LOCK_PIN_MAX: usize = 8;
pub const LOCK_PLAY_COUNT: usize = 8;

fn nearest_row(controls: &[Control], x: f32, tiles: bool) -> Option<Target> {
    let matches_kind = |target: &Target| {
        if tiles {
            matches!(target, Target::HaTile(_))
        } else {
            matches!(target, Target::Input(_))
        }
    };
    let top = controls
        .iter()
        .filter(|control| matches_kind(&control.target))
        .map(|control| control.rect.y)
        .fold(f32::INFINITY, f32::min);
    if !top.is_finite() {
        return None;
    }
    controls
        .iter()
        .filter(|control| matches_kind(&control.target) && (control.rect.y - top).abs() < 1.)
        .min_by(|a, b| {
            (a.rect.center().0 - x)
                .abs()
                .total_cmp(&(b.rect.center().0 - x).abs())
        })
        .map(|control| control.target.clone())
}

impl State {
    pub fn new(preferences: Preferences) -> Self {
        Self {
            page: Page::Home,
            focus: Target::Nav(0),
            inventory: Inventory::default(),
            preferences,
            query: String::new(),
            filter: 0,
            section: 0,
            subpage: None,
            scroll: 0.,
            home_scroll: 0.,
            overlay: None,
            moving: None,
            loading: false,
            reload_artwork: false,
            launching: false,
            notice: None,
            error: None,
            dirty: true,
            pointer: false,
            pressed: None,
            previous_background: None,
            panel_started: None,
            weather: weather::Weather::default(),
            keyboard_field: TextField::Apps,
            greeting_name_draft: String::new(),
            ha_draft: String::new(),
            ha_compose: String::new(),
            tiles: Vec::new(),
            shots: std::collections::BTreeMap::new(),
            shot_errors: std::collections::BTreeMap::new(),
            ring_id: 0,
            pending_watch: None,
            bridge_error: None,
            ha_hold: None,
            pairing: None,
            input_name: String::new(),
            recents: crate::recents::Recents::default(),
            lock: crate::bridge::LockState::default(),
            lock_pin: String::new(),
            lock_status: String::new(),
            lock_status_at: None,
            lock_since: None,
            lock_play_offset: 0,
        }
    }

    pub fn locked(&self) -> bool {
        matches!(self.overlay, Some(Overlay::Lock))
    }

    /// A digit from the remote typed on the lock screen.
    pub fn lock_digit(&mut self, digit: u8) -> Vec<Effect> {
        if !self.locked() {
            return vec![];
        }
        let limit = if self.lock.pin_length > 0 {
            self.lock.pin_length
        } else {
            LOCK_PIN_MAX
        };
        if self.lock_pin.len() < limit {
            self.lock_pin.push(char::from(b'0' + digit.min(9)));
        }
        self.lock_status.clear();
        self.focus = Target::LockPin;
        self.dirty = true;
        vec![]
    }

    fn lock_activate(&mut self) -> Vec<Effect> {
        self.dirty = true;
        if self.focus == Target::LockRequest {
            self.lock_status.clear();
            return vec![Effect::LockRequest];
        }
        if self.lock_pin.is_empty() {
            return vec![];
        }
        vec![Effect::LockUnlock(std::mem::take(&mut self.lock_pin))]
    }

    /// The line under the buttons: our own feedback first, then what Home Assistant says.
    pub fn lock_line(&self) -> String {
        let language = self.preferences.language;
        if !self.lock_status.is_empty() {
            return self.lock_status.clone();
        }
        if self.lock.denied {
            return language.t("lockDenied").to_owned();
        }
        if self.lock.pending {
            return language.t("lockRequestSent").to_owned();
        }
        String::new()
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
        self.dirty = true;
    }

    pub fn tick(&mut self) {
        if self.locked() && !self.preferences.reduced_motion {
            // The toys on the lock screen keep moving.
            self.dirty = true;
        }
        if self
            .lock_status_at
            .is_some_and(|at| at.elapsed() > Duration::from_secs(8))
        {
            self.lock_status.clear();
            self.lock_status_at = None;
            self.dirty = true;
        }
        let recents_open = self.recents_open();
        if recents_open || self.recents.visibility > 0. {
            self.dirty |= self.recents.animate(
                recents_open,
                self.preferences.reduced_motion,
                self.pointer,
                Instant::now(),
            );
        }
        if let Some(start) = self.panel_started {
            self.dirty = true;
            let duration = if self.overlay.is_some() { 120 } else { 150 };
            if self.preferences.reduced_motion || start.elapsed() >= Duration::from_millis(duration)
            {
                self.panel_started = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120))
            && !self.preferences.reduced_motion
        {
            self.dirty = true;
        }
        if let Some((_, start)) = &self.previous_background {
            self.dirty = true;
            if self.preferences.reduced_motion || start.elapsed() >= Duration::from_millis(400) {
                self.previous_background = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() > Duration::from_secs(5))
        {
            self.notice = None;
            self.dirty = true;
        }
    }

    pub fn page_index(&self) -> usize {
        match self.page {
            Page::Home => 0,
            Page::Apps => 1,
            Page::Dom => 2,
            Page::Settings => 3,
        }
    }

    pub fn animating(&self) -> bool {
        ((self.recents_open() || self.recents.visibility > 0.)
            && (self.recents.visibility != if self.recents_open() { 1. } else { 0. }
                || (!self.pointer && self.recents.position != self.recents.selected as f32)))
            || self.previous_background.is_some()
            || self.panel_started.is_some()
            || (self.locked() && !self.preferences.reduced_motion)
            || (!self.preferences.reduced_motion
                && self
                    .notice
                    .as_ref()
                    .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120)))
    }

    pub fn open(&mut self, index: usize) {
        self.weather.cancel_search();
        self.page = match index {
            1 => Page::Apps,
            2 => Page::Dom,
            3 => Page::Settings,
            _ => Page::Home,
        };
        self.focus = Target::Nav(self.page_index());
        self.scroll = 0.;
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        self.query.clear();
        self.filter = 0;
        self.overlay = None;
        self.pairing = None;
        self.moving = None;
        self.subpage = None;
        self.panel_started = None;
        self.dirty = true;
    }

    pub fn apps(&self) -> Vec<&App> {
        let ids = self.preferences.favorites(&self.inventory.apps);
        if self.page == Page::Home {
            return ids
                .iter()
                .filter(|id| !self.preferences.hidden.contains(id))
                .filter_map(|id| self.inventory.apps.iter().find(|app| &app.id == id))
                .collect();
        }
        let query = self.query.to_lowercase();
        let mut apps: Vec<_> = self
            .inventory
            .apps
            .iter()
            .filter(|app| {
                self.preferences.hidden.contains(&app.id) == (self.filter == 5)
                    && (self.filter == 0
                        || self.filter == 5
                        || app.category == CATEGORIES[self.filter - 1])
                    && app.title.to_lowercase().contains(&query)
            })
            .collect();
        apps.sort_by(|a, b| {
            let rank = |app: &App| {
                ids.iter()
                    .position(|id| id == &app.id)
                    .unwrap_or(usize::MAX)
            };
            rank(a)
                .cmp(&rank(b))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        apps
    }

    pub fn content_viewport(&self) -> Rect {
        let bottom = if self.page == Page::Home && self.moving.is_none() {
            1053.6
        } else {
            994.4
        };
        Rect::new(64., 108.8, 1792., bottom - 108.8)
    }

    pub fn favorites_viewport(&self) -> Rect {
        let content = self.content_viewport();
        let left = if self.home_scroll > 0. {
            HOME_FADE_LEFT
        } else {
            0.
        };
        Rect::new(left, content.y, HOME_FADE_RIGHT - left, content.h)
    }

    pub fn hit_test(&self, controls: &[Control], x: f32, y: f32) -> Option<Target> {
        controls
            .iter()
            .rev()
            .find(|control| {
                let viewport =
                    if self.page == Page::Home && matches!(control.target, Target::App(_)) {
                        self.favorites_viewport()
                    } else {
                        self.content_viewport()
                    };
                control.rect.contains(x, y) && (!control.scrollable || viewport.contains(x, y))
            })
            .map(|control| control.target.clone())
    }

    /// Returns whether this wheel event scrolled favorites, so a pending card press is cancelled.
    pub fn wheel_scroll(
        &mut self,
        controls: &[Control],
        position: Option<(f32, f32)>,
        x: i32,
        y: i32,
    ) -> bool {
        let over_favorites = self.page == Page::Home
            && position.is_some_and(|(x, y)| {
                self.favorites_viewport().contains(x, y)
                    && controls.iter().any(|control| {
                        matches!(control.target, Target::App(_)) && control.rect.contains(x, y)
                    })
            });
        let favorites = self.page == Page::Home && (x != 0 || over_favorites);
        if favorites {
            self.scroll_favorites((if x != 0 { x } else { -y }) as f32 * HOME_TILE_PITCH);
        } else {
            let bottom = self.content_viewport().y + self.content_viewport().h - 12.;
            let max = controls
                .iter()
                .filter(|c| c.scrollable)
                .map(|c| c.rect.y + c.rect.h + self.scroll - bottom)
                .fold(0., f32::max);
            self.scroll = (self.scroll - y as f32 * 90.).clamp(0., max.max(0.));
        }
        self.pointer = true;
        self.dirty = true;
        favorites
    }

    pub fn scroll_favorites(&mut self, delta: f32) {
        let count = self
            .preferences
            .favorites(&self.inventory.apps)
            .iter()
            .filter(|id| {
                !self.preferences.hidden.contains(id)
                    && self.inventory.apps.iter().any(|app| &app.id == *id)
            })
            .count();
        let content = if count == 0 {
            0.
        } else {
            (count - 1) as f32 * HOME_TILE_PITCH + HOME_TILE_WIDTH
        };
        let max = (content - (HOME_STRIP_RIGHT - HOME_STRIP_LEFT)).max(0.);
        let next = (self.home_scroll + delta).clamp(0., max);
        if next != self.home_scroll {
            self.home_scroll = next;
            self.dirty = true;
        }
    }

    pub fn set_inventory(&mut self, next: Inventory, warning: Option<String>) {
        self.inventory.apps = next.apps;
        if let Some(warning) = warning {
            self.notify(warning);
        } else {
            self.inventory.inputs = next.inputs;
        }
        self.loading = false;
        self.error = None;
        self.dirty = true;
        self.reconcile_focus();
    }

    pub fn reconcile_focus(&mut self) {
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if let Target::App(id) = &self.focus
            && !self.apps().iter().any(|app| &app.id == id)
        {
            self.focus = self.first_content();
        }
        if let Target::Input(id) = &self.focus
            && !self
                .inventory
                .inputs
                .iter()
                .any(|input| &input.id == id && self.preferences.input_visible(input))
        {
            self.focus = self.first_content();
        }
    }

    pub fn first_content(&self) -> Target {
        match self.page {
            Page::Home => self
                .apps()
                .get(((self.home_scroll - 0.01).max(0.) / HOME_TILE_PITCH).ceil() as usize)
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(if self.error.is_some() {
                    Target::Retry
                } else {
                    Target::Browse
                }),
            Page::Apps => self
                .apps()
                .first()
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(Target::Filter(1)),
            Page::Dom => self
                .ordered_ids()
                .into_iter()
                .next()
                .map(Target::HaTile)
                .unwrap_or(Target::Nav(2)),
            Page::Settings => Target::Sidebar(0),
        }
    }

    pub fn focus(&mut self, target: Target, controls: &[Control]) {
        self.pointer = false;
        self.focus = target;
        self.reveal_focus(controls);
        if let Target::Sidebar(section) = self.focus {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            self.section = section;
            self.subpage = None;
        }
        self.dirty = true;
    }

    /// Remote focus is revealed after layout changes, including hide/reorder.
    /// Pointer paths deliberately do not call this method.
    pub fn reveal_focus(&mut self, controls: &[Control]) -> bool {
        let previous = self.scroll;
        let previous_home = self.home_scroll;
        if let Some(control) = controls.iter().find(|c| c.target == self.focus) {
            if self.page == Page::Home && matches!(self.focus, Target::App(_)) {
                if control.rect.x < HOME_STRIP_LEFT {
                    self.home_scroll += control.rect.x - HOME_STRIP_LEFT;
                } else if control.rect.x + control.rect.w > HOME_STRIP_RIGHT {
                    self.home_scroll += control.rect.x + control.rect.w - HOME_STRIP_RIGHT;
                }
            }
            if matches!(
                self.focus,
                Target::Nav(_)
                    | Target::Filter(_)
                    | Target::ClearFilter
                    | Target::Search
                    | Target::ClearSearch
                    | Target::Sidebar(_)
            ) {
                self.scroll = 0.;
            } else if control.scrollable {
                let top = if self.page == Page::Apps {
                    164.8
                } else {
                    120.8
                };
                if control.rect.y < top {
                    self.scroll = (self.scroll + control.rect.y - top).max(0.);
                }
                let bottom = if self.page == Page::Home && self.moving.is_none() {
                    1041.6
                } else {
                    982.4
                };
                if control.rect.y + control.rect.h > bottom {
                    self.scroll += control.rect.y + control.rect.h - bottom;
                }
            }
        }
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if self.scroll != previous || self.home_scroll != previous_home {
            self.dirty = true;
        }
        self.scroll != previous || self.home_scroll != previous_home
    }

    /// Hover and pointer-down must not move the control before pointer-up.
    pub fn pointer_focus(&mut self, target: Target) {
        if !self.pointer {
            self.dirty = true;
        }
        self.pointer = true;
        if self.focus == target {
            return;
        }
        if let Target::Sidebar(section) = target {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            self.section = section;
            self.subpage = None;
        }
        self.focus = target;
        self.dirty = true;
    }

    pub fn hidden_count(&self) -> usize {
        self.inventory
            .apps
            .iter()
            .filter(|app| self.preferences.hidden.contains(&app.id))
            .count()
    }

    fn section_entry(&self) -> Target {
        match self.section {
            1 => Target::Background(
                BACKGROUNDS
                    .iter()
                    .position(|(id, _)| *id == self.preferences.background)
                    .unwrap_or(3),
            ),
            WEATHER_SECTION => Target::WeatherToggle,
            SYSTEM_SECTION => Target::Setting(3),
            INPUTS_SECTION => self
                .inventory
                .inputs
                .first()
                .map(|input| Target::InputMode(input.id.clone()))
                .unwrap_or(Target::Setting(3)),
            BRIDGE_SECTION => Target::HaToggle,
            _ => Target::Setting(0),
        }
    }

    fn close_subpage(&mut self) {
        if self.subpage == Some(Subpage::WeatherLocation) {
            self.weather.cancel_search();
        }
        self.panel_started = None;
        self.pointer = false;
        self.focus = match self.subpage.take() {
            Some(Subpage::AppNames) => Target::Setting(0),
            Some(Subpage::Greeting) => Target::Setting(6),
            Some(Subpage::Clock) => Target::Setting(1),
            Some(Subpage::ClockFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockFormat
            }
            Some(Subpage::TimeFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockTimeFormat
            }
            Some(Subpage::Input(id)) => Target::InputMode(id),
            Some(Subpage::WeatherLocation) => Target::WeatherLocation,
            Some(Subpage::Temperature) => Target::WeatherTemperature,
            Some(Subpage::Language) => Target::Setting(4),
            None => Target::Sidebar(self.section),
        };
        self.scroll = 0.;
        self.dirty = true;
    }

    pub fn direction(&mut self, direction: Direction, controls: &[Control]) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if self.locked() {
            self.focus = match (direction, &self.focus) {
                (Direction::Down, Target::LockPin) => Target::LockRequest,
                (Direction::Up, Target::LockRequest) => Target::LockPin,
                _ => self.focus.clone(),
            };
            return vec![];
        }
        if self.recents_open() {
            return self.recent_direction(direction);
        }
        if matches!(self.overlay, Some(Overlay::SystemKeyboard)) {
            // The TV keyboard owns its arrows until Done/Back dismisses it.
            return vec![];
        }
        if matches!(self.overlay, Some(Overlay::Camera { .. })) {
            return vec![];
        }
        if let Some(id) = self.moving.clone() {
            let mut ids = self.preferences.favorites(&self.inventory.apps);
            let slots: Vec<_> = ids
                .iter()
                .enumerate()
                .filter_map(|(index, id)| {
                    (!self.preferences.hidden.contains(id)
                        && self.inventory.apps.iter().any(|app| &app.id == id))
                    .then_some(index)
                })
                .collect();
            if let Some(index) = slots.iter().position(|slot| ids[*slot] == id) {
                let next = match direction {
                    Direction::Left => index.saturating_sub(1),
                    Direction::Right => (index + 1).min(slots.len() - 1),
                    _ => index,
                };
                if next != index {
                    ids.swap(slots[index], slots[next]);
                    self.preferences.favorites = Some(ids);
                    self.dirty = true;
                    return vec![Effect::Save];
                }
            }
            return vec![];
        }
        if self.overlay.is_none() {
            if self.page == Page::Home
                && let Target::App(id) = &self.focus
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    match direction {
                        Direction::Left | Direction::Right => {
                            let next = if matches!(direction, Direction::Left) {
                                index.saturating_sub(1)
                            } else {
                                (index + 1).min(apps.len() - 1)
                            };
                            let id = apps[next].id.clone();
                            self.focus(Target::App(id), controls);
                        }
                        Direction::Up => self.focus(Target::Nav(0), controls),
                        Direction::Down => {
                            let x = controls
                                .iter()
                                .find(|control| control.target == self.focus)
                                .map_or(960., |control| control.rect.center().0);
                            let kind_is_tile = controls
                                .iter()
                                .any(|control| matches!(control.target, Target::HaTile(_)));
                            let next = nearest_row(controls, x, kind_is_tile);
                            if let Some(next) = next {
                                self.focus(next, controls);
                            }
                        }
                    }
                    return vec![];
                }
            }
            if self.page == Page::Home
                && let Target::HaTile(id) = &self.focus
            {
                let tiles = self.pinned_ids();
                let index = tiles.iter().position(|tile| tile == id).unwrap_or(0);
                match direction {
                    Direction::Left | Direction::Right => {
                        let next = if matches!(direction, Direction::Left) {
                            index.saturating_sub(1)
                        } else {
                            (index + 1).min(tiles.len().saturating_sub(1))
                        };
                        if let Some(tile) = tiles.get(next) {
                            self.focus(Target::HaTile(tile.clone()), controls);
                        }
                    }
                    Direction::Up => {
                        let apps = self.apps();
                        if let Some(app) = apps.get(index.min(apps.len().saturating_sub(1))) {
                            self.focus(Target::App(app.id.clone()), controls);
                        } else {
                            self.focus(Target::Nav(0), controls);
                        }
                    }
                    Direction::Down => {
                        let x = controls
                            .iter()
                            .find(|control| control.target == self.focus)
                            .map_or(960., |control| control.rect.center().0);
                        if let Some(next) = nearest_row(controls, x, false) {
                            self.focus(next, controls);
                        }
                    }
                }
                return vec![];
            }
            let setting_rows = match self.subpage {
                Some(Subpage::Greeting) => Some([Target::GreetingName, Target::GreetingNameClear]),
                Some(Subpage::AppNames) => Some([Target::AppNamesHome, Target::AppNamesApps]),
                Some(Subpage::Clock) => Some([Target::ClockFormat, Target::ClockTimeFormat]),
                _ => None,
            };
            if let Some(rows) = setting_rows
                && let Some(index) = rows.iter().position(|target| *target == self.focus)
            {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(3)
                        } else {
                            rows[0].clone()
                        },
                        controls,
                    ),
                    Direction::Down if index == 0 => self.focus(rows[1].clone(), controls),
                    Direction::Right
                        if matches!(
                            self.focus,
                            Target::GreetingName | Target::ClockFormat | Target::ClockTimeFormat
                        ) =>
                    {
                        return self.activate();
                    }
                    _ => {}
                }
                return vec![];
            }
            if let Target::Choice(index) = self.focus {
                let count = match self.subpage {
                    Some(Subpage::ClockFormat | Subpage::TimeFormat | Subpage::Temperature) => 2,
                    Some(Subpage::Language) => Language::ALL.len(),
                    _ => 3,
                };
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(3)
                        } else {
                            Target::Choice(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down
                        if index + 1 == count
                            && matches!(self.subpage, Some(Subpage::Input(_))) =>
                    {
                        self.focus(Target::InputRename, controls)
                    }
                    Direction::Down => self.focus(Target::Choice((index + 1) % count), controls),
                    Direction::Right => {}
                }
                return vec![];
            }
            if matches!(self.focus, Target::InputRename | Target::InputNameReset) {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if self.focus == Target::InputNameReset {
                            Target::InputRename
                        } else {
                            Target::Choice(2)
                        },
                        controls,
                    ),
                    Direction::Down
                        if self.focus == Target::InputRename
                            && controls.iter().any(|c| c.target == Target::InputNameReset) =>
                    {
                        self.focus(Target::InputNameReset, controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if let Target::Sidebar(index) = self.focus {
                match direction {
                    Direction::Right => {
                        let entry = self.section_entry();
                        self.focus(entry, controls);
                    }
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(3)
                        } else {
                            Target::Sidebar(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down => {
                        self.focus(Target::Sidebar((index + 1) % SECTION_COUNT), controls)
                    }
                    Direction::Left => {}
                }
                return vec![];
            }
            if matches!(
                self.focus,
                Target::Setting(0 | 1 | 4 | 6)
                    | Target::InputMode(_)
                    | Target::WeatherLocation
                    | Target::WeatherTemperature
            ) && matches!(direction, Direction::Right)
            {
                return self.activate();
            }
            if matches!(self.focus, Target::Setting(_) | Target::WeatherToggle)
                && matches!(direction, Direction::Right)
            {
                return vec![];
            }
            if let Target::InputMode(id) = &self.focus
                && matches!(direction, Direction::Up)
                && self
                    .inventory
                    .inputs
                    .first()
                    .is_some_and(|input| &input.id == id)
            {
                self.focus(Target::Sidebar(INPUTS_SECTION), controls);
                return vec![];
            }
            if self.focus == Target::Setting(0) && matches!(direction, Direction::Up) {
                self.focus(Target::Nav(3), controls);
                return vec![];
            }
            if self.focus == Target::WeatherToggle && matches!(direction, Direction::Up) {
                self.focus(Target::Nav(3), controls);
                return vec![];
            }
            if matches!(
                self.focus,
                Target::WeatherQuery | Target::WeatherSearch | Target::WeatherClear
            ) {
                match direction {
                    Direction::Left => {
                        self.close_subpage();
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(
                            if self.preferences.weather_location.is_some()
                                && self.focus != Target::WeatherClear
                            {
                                Target::WeatherClear
                            } else {
                                Target::Nav(3)
                            },
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Down => {
                        let target = if self.focus == Target::WeatherClear {
                            Some(Target::WeatherQuery)
                        } else if !self.weather.results.is_empty() {
                            Some(Target::WeatherResult(0))
                        } else if self.focus == Target::WeatherQuery {
                            Some(Target::WeatherSearch)
                        } else {
                            None
                        };
                        if let Some(target) = target {
                            self.focus(target, controls);
                        }
                        return vec![];
                    }
                    _ => {}
                }
            }
            if let Target::WeatherResult(index) = self.focus {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::WeatherQuery
                        } else {
                            Target::WeatherResult(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < self.weather.results.len() => {
                        self.focus(Target::WeatherResult(index + 1), controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if self.focus == Target::Search {
                match direction {
                    Direction::Left => {
                        self.focus(
                            Target::Filter(if self.hidden_count() > 0 { 5 } else { 4 }),
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(Target::Nav(1), controls);
                        return vec![];
                    }
                    _ => {}
                }
            }
            if let Target::Nav(index) = self.focus {
                let next = match direction {
                    Direction::Left if index > 0 => Some(index - 1),
                    Direction::Right if index < 3 => Some(index + 1),
                    _ => None,
                };
                if let Some(next) = next {
                    self.open(next);
                    return vec![];
                }
            }
            if self.page == Page::Apps
                && let Target::App(id) = &self.focus
                && matches!(direction, Direction::Up | Direction::Down)
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    // A preceding row may be above the visible scroll region.
                    // Its grid index still defines the route; reveal_focus then
                    // scrolls it into view, without letting the fixed nav steal Up.
                    let next = match direction {
                        Direction::Up if index < APPS_COLUMNS => Some(Target::Filter(1)),
                        Direction::Up => Some(Target::App(apps[index - APPS_COLUMNS].id.clone())),
                        Direction::Down
                            if (index / APPS_COLUMNS + 1) * APPS_COLUMNS < apps.len() =>
                        {
                            Some(Target::App(
                                apps[(index + APPS_COLUMNS).min(apps.len() - 1)].id.clone(),
                            ))
                        }
                        _ => None,
                    };
                    if let Some(next) = next {
                        self.focus(next, controls);
                    }
                    return vec![];
                }
            }
            if let Target::Background(index) = self.focus {
                match direction {
                    Direction::Right if index + 1 < BACKGROUNDS.len() => {
                        self.focus(Target::Background(index + 1), controls);
                        return vec![];
                    }
                    Direction::Left if index % 3 != 0 => {
                        self.focus(Target::Background(index - 1), controls);
                        return vec![];
                    }
                    _ => {}
                }
            }
            if matches!(self.focus, Target::Nav(_)) && matches!(direction, Direction::Down) {
                if self.page == Page::Settings {
                    self.subpage = None;
                    self.section = 0;
                }
                self.focus(self.first_content(), controls);
                return vec![];
            }
            if matches!(
                self.focus,
                Target::HaToggle | Target::HaPair | Target::HaAddress | Target::HaCode
            ) {
                let rows = [
                    Target::HaToggle,
                    Target::HaPair,
                    Target::HaAddress,
                    Target::HaCode,
                ];
                let index = rows
                    .iter()
                    .position(|target| *target == self.focus)
                    .unwrap();
                match direction {
                    Direction::Left => self.focus(Target::Sidebar(BRIDGE_SECTION), controls),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(3)
                        } else {
                            rows[index - 1].clone()
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Right => return self.activate(),
                    _ => {}
                }
                return vec![];
            }
            if self.page == Page::Settings
                && matches!(direction, Direction::Left)
                && !matches!(
                    self.focus,
                    Target::Nav(_) | Target::Sidebar(_) | Target::TvSettings
                )
            {
                if let Target::Background(index) = self.focus {
                    if index % 3 != 0 { /* left follows gallery order */
                    } else {
                        self.focus(Target::Sidebar(self.section), controls);
                        return vec![];
                    }
                } else {
                    self.focus(Target::Sidebar(self.section), controls);
                    return vec![];
                }
            }
        }
        let Some(current) = controls.iter().find(|c| c.target == self.focus) else {
            self.focus(self.first_content(), controls);
            return vec![];
        };
        let (x, y) = current.rect.center();
        let next = controls
            .iter()
            .filter(|candidate| candidate.target != self.focus)
            .filter(|candidate| {
                self.page != Page::Home
                    || !matches!(candidate.target, Target::App(_))
                    || (candidate.rect.x >= HOME_STRIP_LEFT - 0.1
                        && candidate.rect.x + candidate.rect.w <= HOME_STRIP_RIGHT + 0.1)
            })
            .filter_map(|candidate| {
                // Header controls never steal an Up route from the active tab.
                if self.overlay.is_none()
                    && matches!(direction, Direction::Up)
                    && candidate.rect.y < 150.
                    && candidate.target != Target::Nav(self.page_index())
                {
                    return None;
                }
                let (cx, cy) = candidate.rect.center();
                let (primary, cross, overlap) = match direction {
                    Direction::Up => (
                        y - cy,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Down => (
                        cy - y,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Left => (
                        x - cx,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                    Direction::Right => (
                        cx - x,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                };
                (primary > 1.).then_some((
                    candidate.target.clone(),
                    primary + cross * 2. + if cross >= overlap { 10000. } else { 0. },
                ))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(target, _)| target);
        if let Some(next) = next {
            self.focus(next, controls);
        }
        vec![]
    }

    pub fn options(&mut self) {
        if self.overlay.is_some() || self.moving.is_some() {
            return;
        }
        if let Target::App(id) = &self.focus {
            self.overlay = Some(Overlay::Options {
                id: id.clone(),
                restore: self.focus.clone(),
            });
            self.focus = Target::Option(0);
            if !self.preferences.reduced_motion {
                self.panel_started = Some(Instant::now());
            }
            self.dirty = true;
        }
    }

    pub fn hold(&mut self) -> Vec<Effect> {
        if self.locked() {
            return vec![];
        }
        if self.overlay.is_some() {
            return vec![];
        }
        if let Target::HaTile(id) = self.focus.clone()
            && matches!(self.page, Page::Home | Page::Dom)
        {
            return self.toggle_pin(&id);
        }
        if self.page == Page::Home && self.overlay.is_none() {
            if let Target::App(id) = &self.focus {
                self.moving = Some(id.clone());
                self.notice = None;
                self.dirty = true;
            }
        } else {
            self.options();
        }
        vec![]
    }

    pub fn ordered_ids(&self) -> Vec<String> {
        let mut indexed: Vec<(u8, usize, String)> = self
            .tiles
            .iter()
            .enumerate()
            .map(|(index, tile)| (tile.group(), index, tile.entity_id.clone()))
            .collect();
        indexed.sort_by_key(|(group, index, _)| (*group, *index));
        indexed.into_iter().map(|(_, _, id)| id).collect()
    }

    pub fn pinned_ids(&self) -> Vec<String> {
        match &self.preferences.ha_pins {
            None => self
                .tiles
                .iter()
                .map(|tile| tile.entity_id.clone())
                .collect(),
            Some(ids) => ids
                .iter()
                .filter(|id| self.tiles.iter().any(|tile| &tile.entity_id == *id))
                .cloned()
                .collect(),
        }
    }

    fn toggle_pin(&mut self, id: &str) -> Vec<Effect> {
        let mut pins = self.pinned_ids();
        let language = self.preferences.language;
        if let Some(index) = pins.iter().position(|pin| pin == id) {
            pins.remove(index);
            self.notify(language.t("haUnpinned"));
        } else {
            pins.push(id.to_owned());
            self.notify(language.t("haPinned"));
        }
        self.preferences.ha_pins = Some(pins);
        if self.page == Page::Home
            && let Target::HaTile(current) = self.focus.clone()
            && !self.pinned_ids().iter().any(|pin| pin == &current)
        {
            self.focus = self
                .pinned_ids()
                .first()
                .map(|pin| Target::HaTile(pin.clone()))
                .unwrap_or(Target::Nav(0));
        }
        vec![Effect::Save]
    }

    fn anticipate_press(&mut self, id: &str) {
        let Some(tile) = self.tiles.iter_mut().find(|tile| tile.entity_id == id) else {
            return;
        };
        let next = match tile.state.as_str() {
            "on" => "off",
            "off" => "on",
            "open" => "closed",
            "closed" | "closing" => "open",
            "locked" => "unlocked",
            "unlocked" => "locked",
            _ => return,
        };
        tile.state = next.into();
        self.ha_hold = Some((id.into(), next.into(), Instant::now()));
        self.dirty = true;
    }

    pub fn launch_shortcut(&mut self, digit: u8) -> Vec<Effect> {
        if self.locked() {
            return vec![];
        }
        if self.text_input_active() || self.overlay.is_some() || self.moving.is_some() {
            return vec![];
        }
        let Some(id) = self.preferences.shortcuts.get(&digit.to_string()).cloned() else {
            return vec![];
        };
        if self.inventory.apps.iter().any(|app| app.id == id) {
            vec![Effect::Launch(id)]
        } else {
            vec![]
        }
    }

    pub fn activate_shortcut_digit(&mut self, digit: u8) -> Vec<Effect> {
        if self.locked() {
            return vec![];
        }
        let Some(Overlay::Shortcut { id, restore }) = self.overlay.clone() else {
            return vec![];
        };
        const DIGITS: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        let Some(index) = DIGITS.iter().position(|item| *item == digit) else {
            return vec![];
        };
        self.finish_shortcut(index, id, restore)
    }

    fn finish_shortcut(&mut self, index: usize, id: String, restore: Target) -> Vec<Effect> {
        const DIGITS: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        self.overlay = None;
        self.panel_started = None;
        self.focus = restore;
        self.pointer = false;
        self.dirty = true;
        let title = self
            .inventory
            .apps
            .iter()
            .find(|app| app.id == id)
            .map_or(id.clone(), |app| app.title.clone());
        if index == 10 {
            if self.preferences.clear_shortcut(&id) {
                self.notify(
                    self.preferences
                        .language
                        .format("shortcutCleared", &[("app", &title)]),
                );
                return vec![Effect::Save];
            }
            return vec![];
        }
        let Some(digit) = DIGITS.get(index).copied() else {
            return vec![];
        };
        if !self.preferences.assign_shortcut(digit, &id) {
            return vec![];
        }
        self.notify(self.preferences.language.format(
            "shortcutSet",
            &[("key", &digit.to_string()), ("app", &title)],
        ));
        vec![Effect::Save]
    }

    pub fn text(&mut self, text: &str) {
        if self.text_input_active() {
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(120usize.saturating_sub(query.chars().count())),
                );
                self.weather.edit(query);
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::Name {
                let name = &mut self.greeting_name_draft;
                name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(40usize.saturating_sub(name.chars().count())),
                );
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::InputName {
                let length = self.input_name.chars().count();
                self.input_name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(INPUT_NAME_LIMIT.saturating_sub(length)),
                );
                self.dirty = true;
                return;
            }
            if matches!(self.keyboard_field, TextField::HaUrl | TextField::HaCode) {
                let limit: usize = if self.keyboard_field == TextField::HaUrl {
                    200
                } else {
                    64
                };
                let length = self.ha_draft.chars().count();
                self.ha_draft.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(limit.saturating_sub(length)),
                );
                self.ha_compose.clear();
                self.dirty = true;
                return;
            }
            self.query.extend(
                text.chars()
                    .filter(|c| !c.is_control())
                    .take(120usize.saturating_sub(self.query.chars().count())),
            );
            self.scroll = 0.;
            self.dirty = true;
        }
    }

    pub fn delete(&mut self) {
        if self.text_input_active() {
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.pop();
                self.weather.edit(query);
            } else if self.keyboard_field == TextField::Name {
                self.greeting_name_draft.pop();
            } else if self.keyboard_field == TextField::InputName {
                self.input_name.pop();
            } else if matches!(self.keyboard_field, TextField::HaUrl | TextField::HaCode) {
                self.ha_draft.pop();
            } else {
                self.query.pop();
            }
            self.dirty = true;
        }
    }

    pub fn text_input_active(&self) -> bool {
        matches!(
            self.overlay,
            Some(Overlay::Keyboard | Overlay::SystemKeyboard)
        )
    }

    pub fn finish_text_input(&mut self) -> Vec<Effect> {
        if self.text_input_active() {
            self.overlay = None;
            self.panel_started = None;
            self.focus = self.keyboard_return();
            self.pointer = false;
            self.dirty = true;
            return self.commit_text_input();
        }
        vec![]
    }

    fn commit_ha(&mut self) -> Vec<Effect> {
        if !self.ha_compose.is_empty() {
            self.ha_draft
                .push_str(&std::mem::take(&mut self.ha_compose));
        }
        let draft = std::mem::take(&mut self.ha_draft);
        let typed = draft.trim();
        if typed.is_empty() {
            return vec![];
        }
        if self.keyboard_field == TextField::HaUrl {
            let clean = crate::model::clean_ha_url(typed);
            if clean.is_empty() {
                self.notify(self.preferences.language.t("haInvalidAddress"));
                return vec![];
            }
            if self.preferences.ha_url != clean {
                self.preferences.ha_url = clean;
                return vec![Effect::Save, Effect::BridgeRefresh];
            }
        } else if self.keyboard_field == TextField::HaCode {
            let clean = crate::model::clean_ha_token(typed);
            if clean.is_empty() {
                self.notify(self.preferences.language.t("haInvalidCode"));
                return vec![];
            }
            if self.preferences.ha_token != clean {
                self.preferences.ha_token = clean;
                return vec![Effect::Save, Effect::BridgeRefresh];
            }
        }
        vec![]
    }

    pub fn apply_bridge(&mut self, update: crate::bridge::Update) {
        match update {
            crate::bridge::Update::Tiles(mut tiles) => {
                self.bridge_error = None;
                if let Some((id, expect, started)) = self.ha_hold.clone() {
                    if started.elapsed() < Duration::from_secs(4) {
                        if let Some(tile) = tiles.iter_mut().find(|tile| tile.entity_id == id) {
                            if tile.state == expect {
                                self.ha_hold = None;
                            } else {
                                tile.state = expect;
                            }
                        }
                    } else {
                        self.ha_hold = None;
                    }
                }
                if self.tiles != tiles {
                    if let Target::HaTile(id) = &self.focus
                        && !tiles.iter().any(|tile| &tile.entity_id == id)
                    {
                        self.focus = Target::Nav(0);
                    }
                    self.tiles = tiles;
                    self.dirty = true;
                }
            }
            crate::bridge::Update::Lock(lock) => {
                if lock.locked {
                    if !self.locked() {
                        self.overlay = Some(Overlay::Lock);
                        self.lock_since = Some(Instant::now());
                        self.lock_play_offset = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |elapsed| elapsed.subsec_nanos() as usize)
                            % LOCK_PLAY_COUNT;
                        self.moving = None;
                        self.pressed = None;
                        self.lock_pin.clear();
                        self.lock_status.clear();
                    }
                    if !matches!(self.focus, Target::LockPin | Target::LockRequest) {
                        self.focus = Target::LockPin;
                    }
                } else if self.locked() {
                    self.overlay = None;
                    self.lock_since = None;
                    self.lock_pin.clear();
                    self.lock_status.clear();
                    self.focus = Target::Nav(0);
                }
                if !lock.notice.is_empty() && lock.notice != self.lock.notice {
                    self.notify(lock.notice.clone());
                }
                if lock.denied != self.lock.denied || lock.pending != self.lock.pending {
                    self.lock_status.clear();
                    self.lock_status_at = None;
                }
                if self.lock != lock {
                    self.lock = lock;
                }
                self.dirty = true;
            }
            crate::bridge::Update::LockResult {
                request,
                ok,
                retry_after,
                error,
            } => {
                if !self.locked() {
                    return;
                }
                let language = self.preferences.language;
                self.lock_status = if !error.is_empty() {
                    error
                } else if ok {
                    if request {
                        language.t("lockRequestSent").to_owned()
                    } else {
                        String::new()
                    }
                } else if retry_after > 0 {
                    language.format("lockWait", &[("seconds", &retry_after.to_string())])
                } else if request {
                    language.t("lockOffline").to_owned()
                } else {
                    language.t("lockWrongPin").to_owned()
                };
                self.lock_status_at = (!self.lock_status.is_empty()).then(Instant::now);
                if !request {
                    self.lock_pin.clear();
                }
                self.dirty = true;
            }
            crate::bridge::Update::Failed(error) => {
                if self.bridge_error.as_deref() != Some(error.as_str()) {
                    self.bridge_error = Some(error);
                    self.dirty = true;
                }
            }
            crate::bridge::Update::Shot { entity_id, path } => {
                self.shot_errors.remove(&entity_id);
                self.shots.insert(entity_id, path);
                self.dirty = true;
            }
            crate::bridge::Update::ShotFailed { entity_id, error } => {
                if self
                    .shot_errors
                    .get(&entity_id)
                    .is_none_or(|old| old != &error)
                {
                    self.shot_errors.insert(entity_id, error);
                    self.dirty = true;
                }
            }
            crate::bridge::Update::Ring { id, camera, title } => {
                if id > self.ring_id && !camera.is_empty() {
                    self.ring_id = id;
                    if self.locked() {
                        return;
                    }
                    self.moving = None;
                    self.overlay = Some(Overlay::Camera { id: camera.clone() });
                    self.pending_watch = Some(camera);
                    self.notify(title);
                    self.dirty = true;
                }
            }
        }
    }

    fn commit_text_input(&mut self) -> Vec<Effect> {
        if self.keyboard_field == TextField::Name {
            let name = std::mem::take(&mut self.greeting_name_draft);
            let name = name.trim();
            // Empty dismissal preserves the saved value; Clear name is explicit.
            if !name.is_empty() && self.preferences.greeting_name != name {
                self.preferences.greeting_name = name.into();
                return vec![Effect::Save];
            }
            return vec![];
        }
        if matches!(self.keyboard_field, TextField::HaUrl | TextField::HaCode) {
            return self.commit_ha();
        }
        self.commit_input_name()
    }

    /// Closing the keyboard by any route keeps a typed input name. An empty
    /// field leaves the saved name unchanged; Clear name removes it explicitly.
    fn commit_input_name(&mut self) -> Vec<Effect> {
        let name = std::mem::take(&mut self.input_name);
        if self.keyboard_field != TextField::InputName || clean_input_name(&name).is_empty() {
            return vec![];
        }
        let Some(Subpage::Input(id)) = self.subpage.clone() else {
            return vec![];
        };
        if self.preferences.set_input_name(&id, &name) {
            vec![Effect::Save]
        } else {
            vec![]
        }
    }

    pub fn delete_surrounding_text(&mut self, offset: i32, length: i32) {
        if !self.text_input_active() || length <= 0 {
            return;
        }
        // SDL-webOS forwards the native text model's UTF-8 byte offsets,
        // relative to the insertion cursor (at the end of our query).
        let mut query = self.editing_text().to_owned();
        let mut start =
            (query.len() as i64 + i64::from(offset)).clamp(0, query.len() as i64) as usize;
        let mut end = start.saturating_add(length as usize).min(query.len());
        while !query.is_char_boundary(start) {
            start -= 1;
        }
        while !query.is_char_boundary(end) {
            end += 1;
        }
        query.replace_range(start..end, "");
        if self.keyboard_field == TextField::Location {
            self.weather.edit(query);
        } else if self.keyboard_field == TextField::Name {
            self.greeting_name_draft = query;
        } else if self.keyboard_field == TextField::InputName {
            self.input_name = query;
        } else if matches!(self.keyboard_field, TextField::HaUrl | TextField::HaCode) {
            self.ha_draft = query;
        } else {
            self.query = query;
            self.scroll = 0.;
        }
        self.dirty = true;
    }

    pub fn set_ha_compose(&mut self, text: &str) {
        if !matches!(self.keyboard_field, TextField::HaUrl | TextField::HaCode) {
            return;
        }
        let limit: usize = if self.keyboard_field == TextField::HaUrl {
            200
        } else {
            64
        };
        let room = limit.saturating_sub(self.ha_draft.chars().count());
        self.ha_compose = text
            .chars()
            .filter(|c| !c.is_control())
            .take(room)
            .collect();
        self.dirty = true;
    }

    pub fn editing_text(&self) -> &str {
        match self.keyboard_field {
            TextField::Location => &self.weather.query,
            TextField::Name => &self.greeting_name_draft,
            TextField::InputName => &self.input_name,
            TextField::HaUrl | TextField::HaCode => &self.ha_draft,
            TextField::Apps => &self.query,
        }
    }
    pub fn open_keyboard(&mut self, field: TextField) {
        self.keyboard_field = field;
        if field == TextField::Name {
            // SDL-webOS cannot pre-fill the system keyboard's text model.
            self.greeting_name_draft.clear();
        }
        if field == TextField::InputName {
            // The TV keyboard starts empty, so a new name replaces the old one.
            self.input_name.clear();
        }
        if matches!(field, TextField::HaUrl | TextField::HaCode) {
            self.ha_draft.clear();
            self.ha_compose.clear();
        }
        if cfg!(all(
            feature = "webos",
            target_os = "linux",
            target_arch = "arm"
        )) {
            self.overlay = Some(Overlay::SystemKeyboard);
            self.focus = match field {
                TextField::Location => Target::WeatherQuery,
                TextField::Name => Target::GreetingName,
                TextField::InputName => Target::InputRename,
                TextField::HaUrl => Target::HaAddress,
                TextField::HaCode => Target::HaCode,
                TextField::Apps => Target::Search,
            };
            self.panel_started = None;
            self.dirty = true;
            return;
        }
        self.overlay = Some(Overlay::Keyboard);
        self.focus = Target::Key(0);
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn open_location(&mut self) {
        self.weather.edit(String::new());
        self.subpage = Some(Subpage::WeatherLocation);
        self.focus = Target::WeatherQuery;
        self.scroll = 0.;
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn keyboard_return(&self) -> Target {
        match self.keyboard_field {
            TextField::Location => Target::WeatherSearch,
            TextField::Name => Target::GreetingName,
            TextField::InputName => Target::InputRename,
            TextField::HaUrl => Target::HaAddress,
            TextField::HaCode => Target::HaCode,
            TextField::Apps => Target::Search,
        }
    }

    pub fn activate(&mut self) -> Vec<Effect> {
        if self.locked() {
            return self.lock_activate();
        }
        if self.recents_open() {
            use crate::recents::Command;
            return match self.focus.clone() {
                Target::Recent(id) => self.recent_request(Command::Launch(id)),
                Target::RecentRetry => self.recent_request(Command::List),
                _ => vec![],
            };
        }
        if self.moving.take().is_some() {
            // Order was saved at every move. A fresh OK release finishes the
            // mode and keeps the selected favorite, rather than launching it.
            self.dirty = true;
            return vec![];
        }
        self.dirty = true;
        match self.focus.clone() {
            Target::Nav(index) => self.open(index),
            Target::Browse => self.open(1),
            Target::Retry => return vec![Effect::Reload],
            Target::DismissNotice => self.notice = None,
            Target::CloseDialog => return self.back(),
            Target::TvSettings => return vec![Effect::Launch("com.palm.app.settings".into())],
            Target::App(id) | Target::Input(id) if !self.launching => {
                return vec![Effect::Launch(id)];
            }
            Target::HaTile(id) if !self.launching => {
                if self
                    .tiles
                    .iter()
                    .any(|tile| tile.entity_id == id && tile.domain == "camera")
                {
                    self.overlay = Some(Overlay::Camera { id: id.clone() });
                    self.dirty = true;
                    return vec![Effect::Watch(Some(id))];
                }
                self.anticipate_press(&id);
                return vec![Effect::BridgePress(id)];
            }
            Target::HaAddress => self.open_keyboard(TextField::HaUrl),
            Target::HaCode => self.open_keyboard(TextField::HaCode),
            Target::HaPair => return vec![Effect::StartPairing],
            Target::HaPairCancel => {
                self.focus = Target::HaPair;
                return vec![Effect::StopPairing];
            }
            Target::HaToggle => {
                self.preferences.show_ha = !self.preferences.show_ha;
                return vec![Effect::Save];
            }
            Target::Filter(index) => {
                self.filter = if self.filter == index { 0 } else { index };
                self.scroll = 0.;
            }
            Target::ClearFilter => {
                self.filter = 0;
                self.focus = Target::Filter(1);
                self.scroll = 0.;
            }
            Target::ClearSearch => {
                self.query.clear();
                self.focus = Target::Search;
                self.scroll = 0.;
            }
            Target::Search => {
                self.open_keyboard(TextField::Apps);
            }
            Target::WeatherConfigure => {
                self.open(3);
                self.section = WEATHER_SECTION;
                self.open_location();
                self.open_keyboard(TextField::Location);
            }
            Target::WeatherToggle => {
                self.preferences.show_weather = !self.preferences.show_weather;
                self.weather.invalidate_forecast();
                return vec![Effect::Save];
            }
            Target::WeatherLocation => self.open_location(),
            Target::WeatherTemperature => {
                self.subpage = Some(Subpage::Temperature);
                self.focus = Target::Choice(usize::from(
                    self.preferences.temperature_unit == Unit::Fahrenheit,
                ));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::WeatherQuery => self.open_keyboard(TextField::Location),
            Target::GreetingName => self.open_keyboard(TextField::Name),
            Target::GreetingNameClear => {
                self.preferences.greeting_name.clear();
                self.focus = Target::GreetingName;
                return vec![Effect::Save];
            }
            Target::AppNamesHome => {
                self.preferences.show_home_names = !self.preferences.show_home_names;
                return vec![Effect::Save];
            }
            Target::AppNamesApps => {
                self.preferences.show_app_names = !self.preferences.show_app_names;
                return vec![Effect::Save];
            }
            Target::ClockFormat | Target::ClockTimeFormat => {
                let format = self.focus == Target::ClockFormat;
                self.subpage = Some(if format {
                    Subpage::ClockFormat
                } else {
                    Subpage::TimeFormat
                });
                self.focus = Target::Choice(usize::from(if format {
                    self.preferences.hour_24
                } else {
                    self.preferences.show_seconds
                }));
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::InputRename => self.open_keyboard(TextField::InputName),
            Target::InputNameReset => {
                if let Some(Subpage::Input(id)) = &self.subpage
                    && self.preferences.input_names.remove(id).is_some()
                {
                    self.focus = Target::InputRename;
                    return vec![Effect::Save];
                }
            }
            Target::WeatherSearch => {
                return self
                    .weather
                    .search(self.preferences.language)
                    .map(|command| vec![Effect::Weather(command)])
                    .unwrap_or_default();
            }
            Target::WeatherClear => {
                self.preferences.weather_location = None;
                self.weather.remove_location();
                self.focus = Target::WeatherSearch;
                return vec![Effect::Save, Effect::Weather(weather::Command::ClearCache)];
            }
            Target::WeatherResult(index) => {
                if let Some(place) = self.weather.results.get(index).cloned() {
                    self.preferences.weather_location = Some(place);
                    self.weather.invalidate_forecast();
                    self.weather.cancel_search();
                    self.focus = Target::WeatherSearch;
                    return vec![Effect::Save];
                }
            }
            Target::Sidebar(index) => {
                self.section = index;
                self.scroll = 0.;
                self.subpage = None;
                self.focus = self.section_entry();
            }
            Target::Setting(index) => {
                match index {
                    0 => {
                        self.subpage = Some(Subpage::AppNames);
                        self.focus = Target::AppNamesHome;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    1 => {
                        self.subpage = Some(Subpage::Clock);
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        self.focus = Target::ClockFormat;
                        return vec![];
                    }
                    2 => self.preferences.reduced_motion = !self.preferences.reduced_motion,
                    4 => {
                        self.subpage = Some(Subpage::Language);
                        self.focus = Target::Choice(self.preferences.language.index());
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    5 => return vec![Effect::Quit],
                    6 => {
                        self.subpage = Some(Subpage::Greeting);
                        self.focus = Target::GreetingName;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    _ => return vec![Effect::Reload],
                }
                return vec![Effect::Save];
            }
            Target::Background(index) => {
                if !self.preferences.reduced_motion
                    && self.preferences.background != BACKGROUNDS[index].0
                {
                    self.previous_background =
                        Some((self.preferences.background.clone(), Instant::now()));
                }
                self.preferences.background = BACKGROUNDS[index].0.into();
                return vec![Effect::Save];
            }
            Target::InputMode(id) => {
                let mode = self
                    .preferences
                    .input_modes
                    .get(&id)
                    .copied()
                    .unwrap_or_default();
                self.subpage = Some(Subpage::Input(id));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                self.focus = Target::Choice(match mode {
                    InputMode::On => 0,
                    InputMode::Off => 1,
                    InputMode::Auto => 2,
                });
            }
            Target::Choice(index) => {
                match self.subpage.clone() {
                    Some(Subpage::ClockFormat) => self.preferences.hour_24 = index == 1,
                    Some(Subpage::TimeFormat) => self.preferences.show_seconds = index == 1,
                    Some(Subpage::Input(id)) => {
                        self.preferences.input_modes.insert(
                            id,
                            [InputMode::On, InputMode::Off, InputMode::Auto][index.min(2)],
                        );
                    }
                    Some(Subpage::Temperature) => {
                        self.preferences.temperature_unit = if index == 1 {
                            Unit::Fahrenheit
                        } else {
                            Unit::Celsius
                        }
                    }
                    Some(Subpage::Language) => {
                        self.preferences.language =
                            Language::ALL[index.min(Language::ALL.len() - 1)];
                        self.weather.cancel_search();
                    }
                    Some(
                        Subpage::WeatherLocation
                        | Subpage::Greeting
                        | Subpage::AppNames
                        | Subpage::Clock,
                    ) => {
                        return vec![];
                    }
                    None => return vec![],
                }
                self.close_subpage();
                return vec![Effect::Save];
            }
            Target::Option(index) => {
                if let Some(Overlay::Shortcut { id, restore }) = self.overlay.clone() {
                    return self.finish_shortcut(index, id, restore);
                }
                if index == 2
                    && let Some(Overlay::Options { id, restore }) = self.overlay.clone()
                {
                    let focus = self
                        .preferences
                        .digit_for_app(&id)
                        .map(|digit| if digit == 0 { 9 } else { digit as usize - 1 })
                        .unwrap_or(0);
                    self.overlay = Some(Overlay::Shortcut { id, restore });
                    self.focus = Target::Option(focus);
                    self.dirty = true;
                    if !self.preferences.reduced_motion {
                        self.panel_started = Some(Instant::now());
                    }
                    return vec![];
                }
                let Some(Overlay::Options { id, restore }) = self.overlay.take() else {
                    return vec![];
                };
                self.panel_started = None;
                let old_ids: Vec<_> = self.apps().iter().map(|a| a.id.clone()).collect();
                let old_index = old_ids.iter().position(|item| item == &id).unwrap_or(0);
                let mut favorites = self.preferences.favorites(&self.inventory.apps);
                let title = self
                    .inventory
                    .apps
                    .iter()
                    .find(|app| app.id == id)
                    .map_or(id.clone(), |app| app.title.clone());
                if index == 0 {
                    if favorites.contains(&id) {
                        favorites.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteRemoved", &[("app", &title)]),
                        );
                    } else {
                        favorites.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteAdded", &[("app", &title)]),
                        );
                    }
                    self.preferences.hidden.retain(|item| item != &id);
                    self.preferences.favorites = Some(favorites);
                } else if index == 1 {
                    if self.preferences.hidden.contains(&id) {
                        self.preferences.hidden.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("appRestored", &[("app", &title)]),
                        );
                    } else {
                        self.preferences.hidden.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("appHidden", &[("app", &title)]),
                        );
                    }
                }
                if self.filter == 5 && self.hidden_count() == 0 {
                    self.filter = 0;
                }
                self.focus = restore;
                self.pointer = false;
                if let Target::App(id) = &self.focus {
                    let apps = self.apps();
                    if !apps.iter().any(|app| &app.id == id) {
                        self.focus = apps
                            .get(old_index.min(apps.len().saturating_sub(1)))
                            .map(|app| Target::App(app.id.clone()))
                            .unwrap_or_else(|| self.first_content());
                    }
                }
                return if index < 2 {
                    vec![Effect::Save]
                } else {
                    vec![]
                };
            }
            Target::Key(index) => match KEYS[index] {
                "Space" => self.text(" "),
                "Delete" => self.delete(),
                "Clear" => match self.keyboard_field {
                    TextField::Location => self.weather.edit(String::new()),
                    TextField::Name => self.greeting_name_draft.clear(),
                    TextField::InputName => self.input_name.clear(),
                    TextField::HaUrl | TextField::HaCode => self.ha_draft.clear(),
                    TextField::Apps => self.query.clear(),
                },
                "Done" => {
                    self.overlay = None;
                    self.panel_started = None;
                    self.focus = self.keyboard_return();
                    return self.commit_text_input();
                }
                key => self.text(key),
            },
            _ => {}
        }
        vec![]
    }

    pub fn back(&mut self) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if self.locked() {
            self.lock_pin.pop();
            self.lock_status.clear();
            return vec![];
        }
        if self.moving.take().is_some() {
            return vec![];
        }
        if let Some(overlay) = self.overlay.take() {
            self.panel_started = None;
            if matches!(overlay, Overlay::Recents { .. }) {
                self.recents.closing_focus = Some(self.focus.clone());
                self.recents.updated = Instant::now();
                self.pressed = None;
                if self.preferences.reduced_motion {
                    self.recents.visibility = 0.;
                    self.recents.closing_focus = None;
                }
            }
            if let Overlay::Camera { .. } = overlay {
                return vec![Effect::Watch(None)];
            }
            if let Overlay::Shortcut { id, restore } = overlay {
                self.overlay = Some(Overlay::Options { id, restore });
                self.focus = Target::Option(2);
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                return vec![];
            }
            if let Overlay::Options { restore, .. } | Overlay::Recents { restore } = overlay {
                self.focus = restore;
                self.recents.pending = None;
                self.reconcile_focus();
                return vec![];
            }
            self.focus = self.keyboard_return();
            return self.commit_text_input();
        }
        if self.pairing.is_some() && self.page == Page::Settings && self.section == BRIDGE_SECTION {
            self.pairing = None;
            self.focus = Target::HaPair;
            return vec![Effect::StopPairing];
        }
        if self.page == Page::Settings && self.subpage.is_some() {
            self.close_subpage();
            return vec![];
        }
        if self.page == Page::Settings && !matches!(self.focus, Target::Nav(_)) {
            if matches!(self.focus, Target::Sidebar(_)) {
                self.focus = Target::Nav(3);
            } else {
                self.focus = Target::Sidebar(self.section);
            }
            self.scroll = 0.;
            return vec![];
        }
        if self.page == Page::Apps && matches!(self.focus, Target::Search | Target::ClearSearch) {
            self.query.clear();
            self.focus = Target::Nav(1);
            self.scroll = 0.;
            return vec![];
        }
        if self.page != Page::Home {
            self.open(0);
            vec![]
        } else {
            self.open_recents()
        }
    }
}
