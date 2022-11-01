use std::ops::IndexMut;
use std::{cell::RefCell, rc::Rc};

use leptos::*;
use thiserror::Error;
use typed_builder::TypedBuilder;

#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;

#[cfg(feature = "transition")]
use leptos_reactive::use_transition;

use crate::{
    create_location, matching::resolve_path, History, Location, LocationChange, RouteContext,
    RouterIntegrationContext, State,
};

#[cfg(not(feature = "ssr"))]
use crate::{unescape, Url};

#[derive(TypedBuilder)]
pub struct RouterProps {
    #[builder(default, setter(strip_option))]
    base: Option<&'static str>,
    #[builder(default, setter(strip_option))]
    fallback: Option<fn() -> Element>,
    children: Box<dyn Fn() -> Vec<Element>>,
}

#[allow(non_snake_case)]
pub fn Router(cx: Scope, props: RouterProps) -> impl IntoChild {
    // create a new RouterContext and provide it to every component beneath the router
    let router = RouterContext::new(cx, props.base, props.fallback);
    provide_context(cx, router);

    props.children
}

#[derive(Debug, Clone)]
pub struct RouterContext {
    pub(crate) inner: Rc<RouterContextInner>,
}
pub(crate) struct RouterContextInner {
    pub location: Location,
    pub base: RouteContext,
    base_path: String,
    history: Box<dyn History>,
    cx: Scope,
    reference: ReadSignal<String>,
    set_reference: WriteSignal<String>,
    referrers: Rc<RefCell<Vec<LocationChange>>>,
    state: ReadSignal<State>,
    set_state: WriteSignal<State>,
}

impl std::fmt::Debug for RouterContextInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterContextInner")
            .field("location", &self.location)
            .field("base", &self.base)
            .field("history", &std::any::type_name_of_val(&self.history))
            .field("cx", &self.cx)
            .field("reference", &self.reference)
            .field("set_reference", &self.set_reference)
            .field("referrers", &self.referrers)
            .field("state", &self.state)
            .field("set_state", &self.set_state)
            .finish()
    }
}

impl RouterContext {
    pub fn new(cx: Scope, base: Option<&'static str>, fallback: Option<fn() -> Element>) -> Self {
        #[cfg(not(feature = "ssr"))]
        let history = use_context::<RouterIntegrationContext>(cx)
            .unwrap_or_else(|| RouterIntegrationContext(Rc::new(crate::BrowserIntegration {})));
        #[cfg(feature = "ssr")]
        let history = use_context::<RouterIntegrationContext>(cx).expect("You must call provide_context::<RouterIntegrationContext>(cx, ...) somewhere above the <Router/>.");

        // Any `History` type gives a way to get a reactive signal of the current location
        // in the browser context, this is drawn from the `popstate` event
        // different server adapters can provide different `History` implementations to allow server routing
        let source = history.location(cx);

        // if initial route is empty, redirect to base path, if it exists
        let base = base.unwrap_or_default();
        let base_path = resolve_path("", base, None);

        if let Some(base_path) = &base_path && source.with(|s| s.value.is_empty()) {
			history.navigate(&LocationChange {
				value: base_path.to_string(),
				replace: true,
				scroll: false,
				state: State(None)
			});
		}

        // the current URL
        let (reference, set_reference) = create_signal(cx, source.with(|s| s.value.clone()));

        // the current History.state
        let (state, set_state) = create_signal(cx, source.with(|s| s.state.clone()));

        // we'll use this transition to wait for async resources to load when navigating to a new route
        #[cfg(feature = "transition")]
        let transition = use_transition(cx);

        // Each field of `location` reactively represents a different part of the current location
        let location = create_location(cx, reference, state);
        let referrers: Rc<RefCell<Vec<LocationChange>>> = Rc::new(RefCell::new(Vec::new()));

        // Create base route with fallback element
        let base_path = base_path.unwrap_or_default();
        let base = RouteContext::base(cx, &base_path, fallback);

        // Every time the History gives us a new location,
        // 1) start a transition
        // 2) update the reference (URL)
        // 3) update the state
        // this will trigger the new route match below
        create_render_effect(cx, move |_| {
            let LocationChange { value, state, .. } = source();
            cx.untrack(move || {
                if value != reference() {
                    set_reference.update(move |r| *r = value);
                    set_state.update(move |s| *s = state);
                }
            });
        });

        let inner = Rc::new(RouterContextInner {
            base_path: base_path.into_owned(),
            location,
            base,
            history: Box::new(history),
            cx,
            reference,
            set_reference,
            referrers,
            state,
            set_state,
        });

        // handle all click events on anchor tags
        #[cfg(not(feature = "ssr"))]
        leptos_dom::window_event_listener("click", {
            let inner = Rc::clone(&inner);
            move |ev| inner.clone().handle_anchor_click(ev)
        });
        // TODO on_cleanup remove event listener

        Self { inner }
    }

    pub fn pathname(&self) -> Memo<String> {
        self.inner.location.pathname
    }

    pub fn base(&self) -> RouteContext {
        self.inner.base.clone()
    }
}

impl RouterContextInner {
    pub(crate) fn navigate_from_route(
        self: Rc<Self>,
        to: &str,
        options: &NavigateOptions,
    ) -> Result<(), NavigationError> {
        let cx = self.cx;
        let this = Rc::clone(&self);

        // TODO untrack causes an error here
        cx.untrack(move || {
            let resolved_to = if options.resolve {
                this.base.resolve_path(to)
            } else {
                resolve_path("", to, None)
            };

            match resolved_to {
                None => Err(NavigationError::NotRoutable(to.to_string())),
                Some(resolved_to) => {
                    if self.referrers.borrow().len() > 32 {
                        return Err(NavigationError::MaxRedirects);
                    }

                    if resolved_to != (this.reference)() || options.state != (this.state).get() {
                        if cfg!(feature = "server") {
                            // TODO server out
                            self.history.navigate(&LocationChange {
                                value: resolved_to.to_string(),
                                replace: options.replace,
                                scroll: options.scroll,
                                state: options.state.clone(),
                            });
                        } else {
                            {
                                self.referrers.borrow_mut().push(LocationChange {
                                    value: self.reference.get(),
                                    replace: options.replace,
                                    scroll: options.scroll,
                                    state: self.state.get(),
                                });
                            }
                            let len = self.referrers.borrow().len();

                            #[cfg(feature = "transition")]
                            let transition = use_transition(self.cx);
                            //transition.start({
                            let set_reference = self.set_reference;
                            let set_state = self.set_state;
                            let referrers = self.referrers.clone();
                            let this = Rc::clone(&self);
                            //move || {
                            set_reference.update({
                                let resolved = resolved_to.to_string();
                                move |r| *r = resolved
                            });
                            set_state.update({
                                let next_state = options.state.clone();
                                move |state| *state = next_state
                            });
                            if referrers.borrow().len() == len {
                                this.navigate_end(LocationChange {
                                    value: resolved_to.to_string(),
                                    replace: false,
                                    scroll: true,
                                    state: options.state.clone(),
                                })
                                //}
                            }
                            //});
                        }
                    }

                    Ok(())
                }
            }
        })
    }

    pub(crate) fn navigate_end(self: Rc<Self>, mut next: LocationChange) {
        let first = self.referrers.borrow().get(0).cloned();
        if let Some(first) = first {
            if next.value != first.value || next.state != first.state {
                next.replace = first.replace;
                next.scroll = first.scroll;
                self.history.navigate(&next);
            }
            self.referrers.borrow_mut().clear();
        }
    }

    #[cfg(not(feature = "ssr"))]
    pub(crate) fn handle_anchor_click(self: Rc<Self>, ev: web_sys::Event) {
        let ev = ev.unchecked_into::<web_sys::MouseEvent>();
        if ev.default_prevented()
            || ev.button() != 0
            || ev.meta_key()
            || ev.alt_key()
            || ev.ctrl_key()
            || ev.shift_key()
        {
            return;
        }

        let composed_path = ev.composed_path();
        let mut a: Option<web_sys::HtmlAnchorElement> = None;
        for i in 0..composed_path.length() {
            if let Ok(el) = composed_path
                .get(i)
                .dyn_into::<web_sys::HtmlAnchorElement>()
            {
                a = Some(el);
            }
        }
        if let Some(a) = a {
            let href = a.href();
            let target = a.target();

            // let browser handle this event if link has target,
            // or if it doesn't have href or state
            // TODO "state" is set as a prop, not an attribute
            if !target.is_empty() || (href.is_empty() && !a.has_attribute("state")) {
                return;
            }

            let rel = a.get_attribute("rel").unwrap_or_default();
            let mut rel = rel.split([' ', '\t']);

            // let browser handle event if it has rel=external or download
            if a.has_attribute("download") || rel.any(|p| p == "external") {
                return;
            }

            let url = Url::try_from(href.as_str()).unwrap();
            let path_name = unescape(&url.pathname);

            // let browser handle this event if it leaves our domain
            // or our base path
            if url.origin != leptos_dom::location().origin().unwrap_or_default()
                || (!self.base_path.is_empty()
                    && !path_name.is_empty()
                    && !path_name
                        .to_lowercase()
                        .starts_with(&self.base_path.to_lowercase()))
            {
                return;
            }

            let to = path_name + &unescape(&url.search) + &unescape(&url.hash);
            // TODO "state" is set as a prop, not an attribute
            let state = a.get_attribute("state"); // TODO state

            ev.prevent_default();

            if let Err(e) = self.navigate_from_route(
                &to,
                &NavigateOptions {
                    resolve: false,
                    // TODO "replace" is set as a prop, not an attribute
                    replace: a.has_attribute("replace"),
                    scroll: !a.has_attribute("noscroll"),
                    state: State(None), // TODO state
                },
            ) {
                log::error!("{e:#?}");
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum NavigationError {
    #[error("Path {0:?} is not routable")]
    NotRoutable(String),
    #[error("Too many redirects")]
    MaxRedirects,
}

pub struct NavigateOptions {
    pub resolve: bool,
    pub replace: bool,
    pub scroll: bool,
    pub state: State,
}

impl Default for NavigateOptions {
    fn default() -> Self {
        Self {
            resolve: true,
            replace: false,
            scroll: true,
            state: State(None),
        }
    }
}
