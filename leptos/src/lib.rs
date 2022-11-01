//! # About Leptos
//!
//! Leptos is a full-stack framework for building web applications in Rust. You can use it to build
//! - single-page apps (SPAs) rendered entirely in the browser, using client-side routing and loading
//!   or mutating data via async requests to the server
//! - multi-page apps (MPAs) rendered on the server, managing navigation, data, and mutations via
//!   web-standard `<a>` and `<form>` tags
//! - progressively-enhanced multi-page apps ([PEMPAs](https://www.epicweb.dev/the-webs-next-transition)?)
//!   that are rendered on the server and then hydrated on the client, enhancing your `<a>` and `<form>`
//!   navigations and mutations seamlessly when WASM is available.
//!
//! And you can do all three of these **using the same Leptos code.**
//!
//! # `nightly` Note
//! Most of the examples assume you’re using `nightly` Rust. If you’re on stable, note the following:
//! 1. You need to enable the `"stable"` flag in `Cargo.toml`: `leptos = { version = "0.0", features = ["stable"] }`
//! 2. `nightly` enables the function call syntax for accessing and setting signals. If you’re using `stable`,
//!    you’ll just call `.get()`, `.set()`, or `.update()` manually. Check out the
//!    [`counters-stable` example](https://github.com/gbj/leptos/blob/main/examples/counters-stable/src/main.rs)
//!    for examples of the correct API.
//!
//! # Learning by Example
//!  
//! These docs are a work in progress. If you want to see what Leptos is capable of, check out
//! the [examples](https://github.com/gbj/leptos/tree/main/examples):
//! - [`counter`](https://github.com/gbj/leptos/tree/main/examples/counter) is the classic
//!   counter example, showing the basics of client-side rendering and reactive DOM updates
//! - [`counters`](https://github.com/gbj/leptos/tree/main/examples/counter) introduces parent-child
//!   communication via contexts, and the `<For/>` component for efficient keyed list updates.
//! - [`parent-child`](https://github.com/gbj/leptos/tree/main/examples/parent-child) shows four different
//!   ways a parent component can communicate with a child, including passing a closure, context, and more
//! - [`todomvc`](https://github.com/gbj/leptos/tree/main/examples/todomvc) implements the classic to-do
//!   app in Leptos. This is a good example of a complete, simple app. In particular, you might want to
//!   see how we use [create_effect] to [serialize JSON to `localStorage`](https://github.com/gbj/leptos/blob/16f084a71268ac325fbc4a5e50c260df185eadb6/examples/todomvc/src/lib.rs#L164)
//!   and [reactively call DOM methods](https://github.com/gbj/leptos/blob/6d7c36655c9e7dcc3a3ad33d2b846a3f00e4ae74/examples/todomvc/src/lib.rs#L291)
//!   on [references to elements](https://github.com/gbj/leptos/blob/6d7c36655c9e7dcc3a3ad33d2b846a3f00e4ae74/examples/todomvc/src/lib.rs#L254).
//! - [`fetch`](https://github.com/gbj/leptos/tree/main/examples/fetch) introduces
//!   [Resource](leptos_reactive::Resource)s, which allow you to integrate arbitrary `async` code like an
//!   HTTP request within your reactive code.
//! - [`router`](https://github.com/gbj/leptos/tree/main/examples/router) shows how to use Leptos’s nested router
//!   to enable client-side navigation and route-specific, reactive data loading.
//! - [`todomvc`](https://github.com/gbj/leptos/tree/main/examples/todomvc) shows the basics of building an
//!   isomorphic web app. Both the server and the client import the same app code from the `todomvc` example.
//!   The server renders the app directly to an HTML string, and the client hydrates that HTML to make it interactive.
//! - [`hackernews`](https://github.com/gbj/leptos/tree/main/examples/hackernews) pulls everything together.
//!   It integrates calls to a real external REST API, routing, server-side rendering and hydration to create
//!   a fully-functional PEMPA that works as intended even before WASM has loaded and begun to run.
//!
//! (The SPA examples can be run using `trunk serve`. For information about Trunk,
//! [see here]((https://trunkrs.dev/)).)
//!
//! # Quick Links
//!
//! Here are links to the most important sections of the docs:
//! - **Reactivity**: the [leptos_reactive] overview, and more details in
//!   - [create_signal], [ReadSignal], and [WriteSignal] (and [create_rw_signal] and [RwSignal])
//!   - [create_memo] and [Memo]
//!   - [create_resource] and [Resource]
//!   - [create_effect]
//! - **Templating/Views**: the [view] macro
//! - **Routing**: the [leptos_router](https://docs.rs/leptos_router/latest/leptos_router/) crate
//!
//! # Feature Flags
//! - `csr` (*Default*) Client-side rendering: Generate DOM nodes in the browser
//! - `ssr` Server-side rendering: Generate an HTML string (typically on the server)
//! - `hydrate` Hydration: use this to add interactivity to an SSRed Leptos app
//! - `stable` By default, Leptos requires `nightly` Rust, which is what allows the ergonomics
//!   of calling signals as functions. If you need to use `stable`, you will need to call `.get()`
//!   and `.set()` manually.
//! - `serde` (*Default*) In SSR/hydrate mode, uses [serde] to serialize resources and send them
//!   from the server to the client.
//! - `serde-lite` (*Default*) In SSR/hydrate mode, uses [serde-lite] to serialize resources and send them
//!   from the server to the client.
//! - `serde` (*Default*) In SSR/hydrate mode, uses [miniserde] to serialize resources and send them
//!   from the server to the client.
//!
//! **Important Note:** You must enable one of `csr`, `hydrate`, or `ssr` to tell Leptos
//! which mode your app is operating in.
//!
//! # A Simple Counter
//!
//! ```rust
//! use leptos::*;
//!
//! #[component]
//! pub fn SimpleCounter(cx: Scope, initial_value: i32) -> Element {
//!     // create a reactive signal with the initial value
//!     let (value, set_value) = create_signal(cx, initial_value);
//!
//!     // create event handlers for our buttons
//!     // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
//!     let clear = move |_| set_value(0);
//!     let decrement = move |_| set_value.update(|value| *value -= 1);
//!     let increment = move |_| set_value.update(|value| *value += 1);
//!
//!     // this JSX is compiled to an HTML template string for performance
//!     view! {
//!         cx,
//!         <div>
//!             <button on:click=clear>"Clear"</button>
//!             <button on:click=decrement>"-1"</button>
//!             <span>"Value: " {move || value().to_string()} "!"</span>
//!             <button on:click=increment>"+1"</button>
//!         </div>
//!     }
//! }
//! ```
//!
//! Leptos is easy to use with [Trunk](https://trunkrs.dev/) (or with a simple wasm-bindgen setup):
//! ```ignore
//! pub fn main() {
//!     mount_to_body(|cx| view! { cx,  <SimpleCounter initial_value=3 /> })
//! }
//! ```

pub use leptos_core::*;
pub use leptos_dom;
pub use leptos_dom::wasm_bindgen::{JsCast, UnwrapThrowExt};
pub use leptos_dom::*;
pub use leptos_macro::*;
pub use leptos_reactive::*;
pub use leptos_server;
pub use leptos_server::*;

pub use leptos_reactive::debug_warn;

#[cfg(all(feature = "csr", feature = "ssr"))]
compile_error!("leptos features 'csr' and feature 'ssr' cannot be enabled at the same time");

#[cfg(all(feature = "csr", feature = "hydrate"))]
compile_error!("leptos features 'csr' and feature 'hydrate' cannot be enabled at the same time");

#[cfg(all(feature = "hydrate", feature = "ssr"))]
compile_error!("leptos features 'hydrate' and feature 'ssr' cannot be enabled at the same time");
