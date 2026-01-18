//! Utilities for interacting with local LLM runtimes in a strictly linted crate.

// Interdiction stricte de pratiques dangereuses ou non idiomatiques
#![deny(warnings)] // Tous les warnings sont traités comme des erreurs
#![deny(unsafe_code)] // Le code unsafe est interdit
#![deny(missing_docs)] // Toute fonction, struct, enum ou module public doit être documenté
#![deny(dead_code)] // Le code inutilisé est interdit
#![deny(non_camel_case_types)]
// Les types doivent suivre la convention CamelCase (exception explicite possible au besoin)

// Options supplémentaires pour ne rien laisser passer
#![deny(unused_imports)] // Les imports inutilisés sont interdits
#![deny(unused_variables)] // Les variables inutilisés sont interdits
#![deny(unused_must_use)] // Oblige à gérer explicitement les Result et Option
#![deny(non_snake_case)] // Les noms de variables et fonctions doivent être en snake_case
#![deny(non_upper_case_globals)] // Les constantes et globals doivent être en MAJUSCULE
#![deny(nonstandard_style)] // Empêche tout style de code non standard
#![forbid(unsafe_op_in_unsafe_fn)]
// Interdit l'utilisation d'unsafe même dans une fonction unsafe

// Clippy pour stricte discipline
#![deny(clippy::all)] // Active toutes les lints Clippy standard
#![deny(clippy::pedantic)] // Active les lints très strictes de Clippy
#![deny(clippy::nursery)] // Active les lints expérimentales
#![deny(clippy::unwrap_used)] // Interdit unwrap()
#![deny(clippy::expect_used)] // Interdit expect()
#![deny(clippy::panic)] // Interdit panic!()
#![deny(clippy::print_stdout)] // Interdit println!() en production
#![deny(clippy::todo)] // Interdit les TODO dans le code
#![deny(clippy::unimplemented)] // Interdit les fonctions non implémentées
#![deny(clippy::missing_const_for_fn)] // Force const lorsque possible
#![deny(clippy::unwrap_in_result)] // Interdit unwrap() sur Result
#![deny(clippy::module_inception)] // Interdit un module ayant le même nom que le crate
#![deny(clippy::redundant_clone)] // Interdit les clones inutiles
#![deny(clippy::shadow_unrelated)] // Interdit le shadowing de variables non liées
#![deny(clippy::too_many_arguments)] // Limite le nombre d’arguments des fonctions
#![deny(clippy::cognitive_complexity)] // Limite la complexité cognitive des fonctions

// Lints pour sécurité et robustesse
#![deny(overflowing_literals)] // Interdit les littéraux qui débordent

/// LLM-focused components, including Ollama helpers.
pub mod llm;
/// Long-term memory components (`SQLite`, retrieval, summarization).
pub mod memory;
/// HTTP server and API routes.
#[allow(
    clippy::missing_errors_doc,
    clippy::missing_const_for_fn,
    clippy::unused_async
)]
pub mod server;
/// Web scraping and search components.
#[allow(
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::redundant_closure_for_method_calls,
    clippy::collapsible_if,
    clippy::manual_strip,
    clippy::cast_possible_truncation,
    clippy::needless_raw_string_hashes,
    clippy::cognitive_complexity,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::missing_const_for_fn,
    clippy::shadow_unrelated,
    clippy::unnecessary_map_or,
    clippy::cast_precision_loss,
    clippy::redundant_closure,
    clippy::option_if_let_else,
    clippy::map_unwrap_or,
    clippy::missing_errors_doc
)]
pub mod scraping;
/// Entry helpers to start the Halldyll agent.
pub mod start_halldyll_agent;
