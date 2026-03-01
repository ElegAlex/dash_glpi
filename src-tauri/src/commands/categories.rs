use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::db::queries;
use crate::state::{AppState, DbAccess};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriesRequest {
    pub scope: String,
    pub source: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTree {
    pub source: String,
    pub nodes: Vec<CategoryNode>,
    pub total_tickets: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryNode {
    pub name: String,
    pub full_path: String,
    pub level: usize,
    pub count: usize,
    pub percentage: f64,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen: f64,
    pub children: Vec<CategoryNode>,
}

fn make_node(
    full_path: &str,
    agg: &HashMap<String, (usize, usize, usize)>,
    total: usize,
) -> CategoryNode {
    let parts: Vec<&str> = full_path.split(" > ").collect();
    let level = parts.len();
    let name = parts.last().copied().unwrap_or("").to_string();
    let (count, incidents, demandes) = agg.get(full_path).copied().unwrap_or((0, 0, 0));

    let percentage = if total == 0 {
        0.0
    } else {
        (count as f64 / total as f64 * 1000.0).round() / 10.0
    };

    let children = if level < 3 {
        let prefix = format!("{} > ", full_path);
        let mut child_paths: Vec<String> = agg
            .keys()
            .filter(|k| k.starts_with(&prefix) && k[prefix.len()..].find(" > ").is_none())
            .cloned()
            .collect();
        child_paths.sort_by(|a, b| agg[b].0.cmp(&agg[a].0));
        child_paths
            .into_iter()
            .map(|p| make_node(&p, agg, total))
            .collect()
    } else {
        vec![]
    };

    CategoryNode {
        name,
        full_path: full_path.to_string(),
        level,
        count,
        percentage,
        incidents,
        demandes,
        age_moyen: 0.0,
        children,
    }
}

fn build_tree_nodes(
    raw: Vec<(String, i64, i64, i64)>,
    total: usize,
) -> Vec<CategoryNode> {
    let mut agg: HashMap<String, (usize, usize, usize)> = HashMap::new();

    for (path, count, incidents, demandes) in &raw {
        let parts: Vec<&str> = path.split(" > ").collect();
        for depth in 1..=parts.len().min(3) {
            let ancestor = parts[..depth].join(" > ");
            let entry = agg.entry(ancestor).or_insert((0, 0, 0));
            entry.0 += *count as usize;
            entry.1 += *incidents as usize;
            entry.2 += *demandes as usize;
        }
    }

    let mut roots: Vec<String> = agg
        .keys()
        .filter(|k| !k.contains(" > "))
        .cloned()
        .collect();
    roots.sort_by(|a, b| agg[b].0.cmp(&agg[a].0));

    roots
        .into_iter()
        .map(|p| make_node(&p, &agg, total))
        .collect()
}

#[tauri::command]
pub async fn get_categories_tree(
    state: tauri::State<'_, AppState>,
    request: CategoriesRequest,
) -> Result<CategoryTree, String> {
    state.db(|conn| {
        let raw = queries::get_category_tree_data(conn)?;
        let total: usize = raw.iter().map(|(_, c, _, _)| *c as usize).sum();
        let nodes = build_tree_nodes(raw, total);
        let source = request.source.unwrap_or_else(|| request.scope.clone());
        Ok(CategoryTree {
            source,
            nodes,
            total_tickets: total,
        })
    })
}
