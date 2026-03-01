pub mod bilan;
pub mod classifier;
pub mod stock;
pub mod temporal;

pub use classifier::{classify_ticket, poids_priorite};
pub use stock::{
    compute_age_distribution, compute_couleur_seuil, compute_median, enrich_technician_stock,
};
