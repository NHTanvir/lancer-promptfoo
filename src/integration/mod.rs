//! Integration layer: bridges to the upstream promptfoo CLI (subprocess) and
//! the Lancer ecosystem (HTTP capture shim, OAST correlation hooks).

pub mod promptfoo;
pub mod shim;
