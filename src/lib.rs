//! `iopole` — outil en ligne de commande pour interagir avec la plateforme Iopole.
//!
//! La logique vit dans la bibliothèque afin d'être testable indépendamment
//! du binaire, qui se contente de parser les arguments.

mod cli;

pub use cli::Cli;
