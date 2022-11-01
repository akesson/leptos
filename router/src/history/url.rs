use crate::ParamsMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Url {
    pub origin: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

impl Url {
    pub fn search_params(&self) -> ParamsMap {
        let map = self
            .search
            .trim_start_matches('?')
            .split('&')
            .filter_map(|piece| {
                let mut parts = piece.split('=');
                let (k, v) = (parts.next(), parts.next());
                match k {
                    Some(k) if !k.is_empty() => {
                        Some((unescape(k), unescape(v.unwrap_or_default())))
                    }
                    _ => None,
                }
            })
            .collect::<linear_map::LinearMap<String, String>>();
        ParamsMap(map)
    }
}

#[cfg(feature = "ssr")]
pub fn unescape(s: &str) -> String {
    urlencoding::decode(s)
        .unwrap_or_else(|_| std::borrow::Cow::from(s))
        .replace('+', " ")
}

#[cfg(not(feature = "ssr"))]
pub fn unescape(s: &str) -> String {
    js_sys::decode_uri(s).unwrap().into()
}

#[cfg(feature = "ssr")]
pub fn escape(s: &str) -> String {
    urlencoding::encode(s).into()
}

#[cfg(not(feature = "ssr"))]
pub fn escape(s: &str) -> String {
    js_sys::encode_uri(s).as_string().unwrap()
}

#[cfg(not(feature = "ssr"))]
impl TryFrom<&str> for Url {
    type Error = String;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let fake_host = String::from("http://leptos");
        let url = web_sys::Url::new_with_base(url, &fake_host)
            .map_err(|e| e.as_string().unwrap_or_default())?;
        Ok(Self {
            origin: url.origin(),
            pathname: url.pathname(),
            search: url.search(),
            hash: url.hash(),
        })
    }
}

#[cfg(feature = "ssr")]
impl TryFrom<&str> for Url {
    type Error = String;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = url::Url::parse(url).map_err(|e| e.to_string())?;
        Ok(Self {
            origin: url.origin().unicode_serialization(),
            pathname: url.path().to_string(),
            search: url.query().unwrap_or_default().to_string(),
            hash: Default::default(),
        })
    }
}
