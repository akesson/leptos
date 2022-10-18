use std::{any::Any, fmt::Debug, rc::Rc};

use leptos::*;
use serde::{de::DeserializeOwned, Serialize};

use crate::use_route;

// SSR and CSR both do the work in their own environment and return it as a resource
#[cfg(not(feature = "hydrate"))]
pub fn use_loader<S, T>(cx: Scope) -> Resource<S, T>
where
    S: PartialEq + Debug + Clone + 'static,
    T: Debug + Clone + Serialize + DeserializeOwned + 'static,
{
    let route = use_route(cx);
    let mut loader = route.loader().clone().unwrap_or_else(|| {
        debug_warn!(
            "use_loader() called on a route without a loader: {:?}",
            route.path()
        );
        panic!()
    });

    let id = match loader.resource {
        Some(id) => id,
        None => {
            let id = (loader.factory)(cx);
            loader.resource = Some(id);
            id
        }
    };

    Resource::from_id(cx, id)
}

// In hydration mode, only run the loader on the server
#[cfg(feature = "hydrate")]
pub fn use_loader<T>(cx: Scope) -> Resource<(ParamsMap, Url), T>
where
    T: Clone + Debug + Serialize + DeserializeOwned + 'static,
{
    use wasm_bindgen::{JsCast, UnwrapThrowExt};

    use crate::use_query_map;

    let route = use_route(cx);
    let params = use_params_map(cx);

    let location = use_location(cx);
    let route = use_route(cx);
    let url = move || Url {
        origin: String::default(), // don't care what the origin is for this purpose
        pathname: route.path().into(), // only use this route path, not all matched routes
        search: location.search.get(), // reload when any of query string changes
        hash: String::default(),   // hash is only client-side, shouldn't refire
    };

    create_resource(
        cx,
        move || (params.get(), url()),
        move |(params, url)| async move {
            log::debug!("[LOADER] calling loader; should fire whenever params or URL change");

            let route = use_route(cx);
            let query = use_query_map(cx);

            let mut opts = web_sys::RequestInit::new();
            opts.method("GET");
            let url = format!("{}{}", route.path(), query.get().to_query_string());

            let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap_throw();
            request
                .headers()
                .set("Accept", "application/json")
                .unwrap_throw();

            let window = web_sys::window().unwrap_throw();
            let resp_value =
                wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
                    .await
                    .unwrap_throw();
            let resp = resp_value.unchecked_into::<web_sys::Response>();
            let text = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap_throw())
                .await
                .unwrap_throw()
                .as_string()
                .unwrap_throw();
            //let decoded = window.atob(&text).unwrap_throw();
            //bincode::deserialize(&decoded.as_bytes()).unwrap_throw()
            //serde_json::from_str(&text.as_string().unwrap_throw()).unwrap_throw()
            serde_json::from_str(&text).unwrap_throw()
        },
    )
}

pub trait AnySerialize {
    fn serialize(&self) -> Option<String>;

    fn as_any(&self) -> &dyn Any;
}

impl<T> AnySerialize for T
where
    T: Any + Serialize + 'static,
{
    fn serialize(&self) -> Option<String> {
        serde_json::to_string(&self).ok()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct Loader {
    pub resource: Option<ResourceId>,
    pub factory: Rc<dyn Fn(Scope) -> ResourceId>,
    #[cfg(feature = "ssr")]
    pub serializer: Rc<dyn Fn(Scope, ResourceId) -> Pin<Box<dyn Future<Output = String>>>>,
}

impl<F, S, T> From<F> for Loader
where
    F: Fn(Scope) -> Resource<S, T> + 'static,
    S: PartialEq + Debug + Clone + 'static,
    T: Debug + Clone + Serialize + DeserializeOwned + 'static,
{
    fn from(factory: F) -> Self {
        Loader {
            resource: None,
            factory: Rc::new(move |cx| factory(cx).id),
            #[cfg(feature = "ssr")]
            serializer: Rc::new(move |cx, id| {
                let res = Resource::<S, T>::from_id(cx, id);
                let fut = res.to_future(cx);
                Box::pin(async move {
                    let val = fut.await;
                    serde_json(&val).unwrap()
                })
            }),
        }
    }
}

impl std::fmt::Debug for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loader").finish()
    }
}

#[cfg(feature = "ssr")]
pub async fn loader_to_json(view: impl Fn(Scope) -> String + 'static) -> Option<String> {
    let (data, _, disposer) = run_scope_undisposed(move |cx| async move {
        let _shell = view(cx);

        let mut route = use_context::<crate::RouteContext>(cx)?;
        // get the innermost route matched by this path
        while let Some(child) = route.child() {
            route = child;
        }
        let json = route
            .loader()
            .as_ref()
            .map(|loader| loader.serializer(cx))
            .await;

        data.await.serialize()
    });
    let data = data.await;
    disposer.dispose();
    data
}
