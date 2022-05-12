#![feature(box_patterns)]

mod bundle;
mod bundle_context;
mod chunk;
mod dependency_scanner;
pub mod hmr;
mod js_module;
mod module_graph;
mod options;
mod plugin;
mod plugin_driver;
mod task;
mod utils;
pub use ast;
pub use bundle::*;
pub use bundle_context::*;
pub use chunk::*;
pub use js_module::*;
pub use module_graph::*;
use once_cell::sync::Lazy;
pub use options::*;
pub use plugin::*;
pub use plugin_driver::*;
use swc_common::Globals;
pub use utils::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ResolvedId {
  pub path: String,
  pub external: bool,
}

impl ResolvedId {
  pub fn new<T: Into<String>>(path: T, external: bool) -> Self {
    Self {
      path: path.into(),
      external,
      // module_side_effects: false,
    }
  }
}

pub static SWC_GLOBALS: Lazy<Globals> = Lazy::new(|| Globals::new());
