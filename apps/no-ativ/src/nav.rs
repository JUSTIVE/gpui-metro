//! The page stack: what can be on it, and the handle pages use to act on the
//! app from inside their event handlers.

use gpui::{
    AnyElement, App, ClickEvent, ElementId, Entity, SharedString, WeakEntity, Window, prelude::*,
};
use gpui_metro::{AppBar, TextInput, motion};

use crate::app::NoAtiv;
use crate::data::{IncidentArea, IncidentSeverity};

/// One entry of the back stack. Per-page view state (the selected pivot
/// item, a half-filled form) lives on the entry, so going back restores it.
pub enum Page {
    Start,
    Runs { pivot: usize },
    Run { id: String },
    Telemetry { zone: usize },
    Strains,
    Strain { id: String },
    Incidents { pivot: usize },
    Incident { id: String },
    NewIncident(Box<IncidentForm>),
    Reagents { pivot: usize },
    Shipments { pivot: usize },
    Freezer,
    Deck { pivot: usize },
    People { pivot: usize },
    Settings,
}

impl Page {
    /// The selected pivot item or panorama section, for pages that have one.
    pub fn selected_mut(&mut self) -> Option<&mut usize> {
        match self {
            Page::Runs { pivot }
            | Page::Incidents { pivot }
            | Page::Reagents { pivot }
            | Page::Shipments { pivot }
            | Page::Deck { pivot }
            | Page::People { pivot } => Some(pivot),
            Page::Telemetry { zone } => Some(zone),
            _ => None,
        }
    }

    pub fn form_mut(&mut self) -> Option<&mut IncidentForm> {
        match self {
            Page::NewIncident(form) => Some(form),
            _ => None,
        }
    }
}

pub const AREAS: [IncidentArea; 6] = [
    IncidentArea::Bioreactor,
    IncidentArea::Freezer,
    IncidentArea::Media,
    IncidentArea::Sequencing,
    IncidentArea::Safety,
    IncidentArea::Facility,
];
pub const SEVERITIES: [IncidentSeverity; 3] = [
    IncidentSeverity::Minor,
    IncidentSeverity::Major,
    IncidentSeverity::Critical,
];

/// The new-incident form's state.
pub struct IncidentForm {
    pub title: Entity<TextInput>,
    pub area: usize,
    pub severity: usize,
    pub link_run: bool,
    /// Runs the "linked run" picker offers, by id.
    pub run_choices: Vec<String>,
    pub run: usize,
    pub quarantine: bool,
    pub page_lead: bool,
}

impl IncidentForm {
    /// A blank form, or one pre-linked to `run_id` (flagging from a run).
    pub fn new(run_choices: Vec<String>, run_id: Option<&str>, cx: &mut App) -> Self {
        let mut run_choices = run_choices;
        if let Some(id) = run_id
            && !run_choices.iter().any(|r| r == id)
        {
            run_choices.insert(0, id.to_string());
        }
        let run = run_id
            .and_then(|id| run_choices.iter().position(|r| r == id))
            .unwrap_or(0);
        Self {
            title: cx.new(|cx| TextInput::new("what happened?", cx)),
            area: 0,
            severity: if run_id.is_some() { 1 } else { 0 },
            link_run: run_id.is_some(),
            run_choices,
            run,
            quarantine: false,
            page_lead: false,
        }
    }
}

/// What a page renders: its body, and optionally an application bar.
pub struct PageView {
    pub body: AnyElement,
    pub app_bar: Option<AppBar>,
}

impl PageView {
    pub fn new(body: impl IntoElement) -> Self {
        Self {
            body: body.into_any_element(),
            app_bar: None,
        }
    }
    pub fn app_bar(mut self, bar: AppBar) -> Self {
        self.app_bar = Some(bar);
        self
    }
}

/// Lets page code reach the app from event handlers, and keys entrance
/// animations to the current navigation so they replay on every visit.
#[derive(Clone)]
pub struct Nav {
    app: WeakEntity<NoAtiv>,
    generation: usize,
    direction: motion::Direction,
}

impl Nav {
    pub fn new(app: WeakEntity<NoAtiv>, generation: usize, direction: motion::Direction) -> Self {
        Self {
            app,
            generation,
            direction,
        }
    }

    /// Adapts an app-mutating closure to a component callback that hands
    /// over a value (a pivot index, a toggle state, a slider position…).
    pub fn on<A: 'static>(
        &self,
        f: impl Fn(&mut NoAtiv, A, &mut Window, &mut Context<NoAtiv>) + 'static,
    ) -> impl Fn(A, &mut Window, &mut App) + 'static {
        let app = self.app.clone();
        move |a, window, cx| {
            app.update(cx, |this, cx| f(this, a, window, cx)).ok();
        }
    }

    /// Same as [`Nav::on`], for click handlers.
    pub fn click(
        &self,
        f: impl Fn(&mut NoAtiv, &mut Window, &mut Context<NoAtiv>) + 'static,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let app = self.app.clone();
        move |_, window, cx| {
            app.update(cx, |this, cx| f(this, window, cx)).ok();
        }
    }

    /// A click handler that pushes the page `page` builds.
    pub fn push(
        &self,
        page: impl Fn() -> Page + 'static,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        self.click(move |this, window, cx| this.push(page(), window, cx))
    }

    /// The `on_select` of the top page's pivot or panorama.
    pub fn select(&self) -> impl Fn(usize, &mut Window, &mut App) + 'static {
        self.on(|this, ix, _, cx| {
            this.update_page(|page| {
                if let Some(selected) = page.selected_mut() {
                    *selected = ix;
                }
            });
            cx.notify();
        })
    }

    /// Feathers list items in after a navigation: item `ix` arrives
    /// `ix × FEATHER` after the first. Items past the first screen skip it.
    pub fn feather(&self, key: &str, ix: usize, child: impl IntoElement, cx: &App) -> AnyElement {
        if ix >= 10 {
            return child.into_any_element();
        }
        let id = ElementId::from((SharedString::from(format!("{key}-{}", self.generation)), ix));
        motion::enter(id, self.direction, ix + 1, child, cx)
    }
}
