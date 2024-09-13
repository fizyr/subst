#[cfg(feature = "indexmap")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "indexmap")))]
mod indexmap;

#[cfg(feature = "json")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "json")))]
pub mod json;

#[cfg(feature = "yaml")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "yaml")))]
pub mod yaml;

#[cfg(feature = "toml")]
#[cfg_attr(feature = "doc-cfg", doc(cfg(feature = "toml")))]
pub mod toml;
