# Segment 3 — KPI Stock + Bilan temporel : calculs et agrégation

**Guide technique complet pour le moteur d'indicateurs du GLPI Dashboard**

---

Le module KPI constitue le cœur analytique de l'application. Il transforme les données brutes parsées (Segment 1) et stockées en SQLite (Segment 2) en indicateurs actionnables pour le pilotage du stock de tickets. Ce guide couvre l'intégralité de la logique de calcul : classification vivant/terminé, statistiques d'âge, distribution par tranches, analyse de charge technicien, flux temporels entrée/sortie, et ventilation multi-dimensionnelle. L'architecture retenue est un **modèle hybride SQL + Rust** : les comptages, sommes et agrégations groupées s'exécutent en SQLite (scan unique, pas de transfert de données), tandis que les médianes, percentiles et scores composites se calculent en mémoire sur des `Vec<f64>` Rust. Pour ≤ 50 000 tickets, l'ensemble des KPI du tableau de bord se calcule en **moins de 5 ms**.

---

## 1. Référence des statuts GLPI 9.5 et classification vivant/terminé

### 1.1 Les six statuts standard

GLPI 9.5 définit exactement **six statuts de ticket** sous forme de constantes PHP dans `CommonITILObject`. Ces statuts sont codés en dur (non configurables en base) et apparaissent dans les exports CSV sous forme de libellés français, pas de codes numériques.

|Code|Constante PHP|Libellé CSV français|Classification|
|:-:|---|---|:-:|
|**1**|`INCOMING`|`Nouveau`|**Vivant**|
|**2**|`ASSIGNED`|`En cours (Attribué)`|**Vivant**|
|**3**|`PLANNED`|`En cours (Planifié)`|**Vivant**|
|**4**|`WAITING`|`En attente`|**Vivant**|
|**5**|`SOLVED`|`Résolu`|**Terminé**|
|**6**|`CLOSED`|`Clos`|**Terminé**|

Le champ `est_vivant` dans la table `tickets` correspond directement à la méthode interne GLPI `getNotSolvedStatusArray()`, qui retourne les statuts 1 à 4. **Les statuts 5 (Résolu) et 6 (Clos) sont tous deux "terminé"**, bien qu'ITIL distingue la résolution (le technicien déclare le correctif) de la clôture (le demandeur confirme ou la clôture automatique se déclenche après un délai configurable).

### 1.2 Données réelles de l'export CPAM 92

L'analyse du fichier `tickets.csv` (9 616 tickets) révèle la distribution suivante :

|Statut|Nombre|%|Classification|
|---|--:|--:|:-:|
|Clos|9 070|94,3%|Terminé|
|En cours (Attribué)|330|3,4%|Vivant|
|En attente|187|1,9%|Vivant|
|En cours (Planifié)|23|0,24%|Vivant|
|Résolu|3|0,03%|Terminé|
|Nouveau|3|0,03%|Vivant|
|**Total vivants**|**543**|**5,6%**||
|**Total terminés**|**9 073**|**94,4%**||

**Observations clés :**

- La quasi-totalité des terminés sont `Clos` (99,97%), ce qui indique que la clôture automatique GLPI fonctionne correctement après résolution
- Seulement 3 tickets `Résolu` non encore clos — la fenêtre de transition Résolu → Clos est très courte
- Le ratio vivants/terminés (~5,6%) est sain pour un export combinant stock et historique

### 1.3 Priorités réelles

L'export contient **sept valeurs de priorité**, dont une non standard :

|Priorité|Total|Dont vivants|Poids recommandé|
|---|--:|:-:|:-:|
|Moyenne|5 163|268|3|
|Haute|3 802|223|5|
|Basse|590|47|2|
|Très basse|37|5|1|
|Très haute|17|0|8|
|Majeure|7|0|10|

**`Majeure` est une priorité non standard** — elle n'existe pas dans le GLPI vanilla 9.5 (qui s'arrête à Très haute). Elle a probablement été ajoutée via personnalisation GLPI locale ou plugin. Les 7 tickets concernés sont tous `Clos`, tous de type `Demande`, avec urgence `Très haute`. Le parser doit accepter cette valeur sans erreur et lui attribuer le poids le plus élevé.

### 1.4 Groupes de techniciens — structure hiérarchique réelle

Les groupes suivent une hiérarchie à 2-3 niveaux séparés par `>` :

|Groupe complet|Niveau 1|Niveau 2|Niveau 3|Tickets|
|---|---|---|---|--:|
|`_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`|_DSI|_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL|—|6 562|
|`_DSI > _PRODUCTION-INFRASTRUCTURES`|_DSI|_PRODUCTION-INFRASTRUCTURES|—|1 474|
|`_DSI > _SERVICE DES CORRESPONDANTS INFORMATIQUE`|_DSI|_SERVICE DES CORRESPONDANTS INFORMATIQUE|—|1 178|
|`_DSI > _HABILITATIONS_PRODUCTION`|_DSI|_HABILITATIONS_PRODUCTION|—|302|
|`_DSI > _SUPPORT ... > _SUPPORT - PARC`|_DSI|_SUPPORT UTIL...|_SUPPORT - PARC|165|
|`_DSI > _DIADEME`|_DSI|_DIADEME|—|31|
|`_DSI > _DEVELOPPEMENT & INDUSTRIALISATION`|_DSI|_DEVELOPPEMENT & INDUSTRIALISATION|—|24|
|`GC_SD`|GC_SD|—|—|1|

**Attention** : le champ `Attribué à - Groupe de techniciens` peut être multilignes (plusieurs groupes assignés séparés par `\n`). Le parsing split sur `\n` puis chaque ligne est découpée sur `>` pour extraire les niveaux hiérarchiques. Le caractère `&amp;` dans `_DEVELOPPEMENT & INDUSTRIALISATION` est une entité HTML — le crate csv le désérialise tel quel en `&amp;`, il faut le décoder en `&` lors de la normalisation.

### 1.5 Code Rust de classification

```rust
/// Classifie un statut GLPI comme vivant (true) ou terminé (false).
/// Gère les 6 statuts standard GLPI 9.5.
/// Les statuts inconnus sont classés comme vivants par sécurité
/// (mieux vaut suivre un ticket de trop que d'en oublier un).
pub fn est_vivant(statut: &str) -> bool {
    !matches!(statut.trim(), "Clos" | "Résolu")
}

/// Retourne le poids de pondération pour une priorité GLPI.
/// Inclut "Majeure" (non standard, présent dans l'export CPAM 92).
/// Pondération exponentielle : un P1 consomme plus de ressources que cinq P4.
pub fn poids_priorite(priorite: &str) -> f64 {
    match priorite.trim() {
        "Majeure"    => 10.0,
        "Très haute" => 8.0,
        "Haute"      => 5.0,
        "Moyenne"    => 3.0,
        "Basse"      => 2.0,
        "Très basse" => 1.0,
        _            => 1.0,  // Valeur inconnue → poids minimal
    }
}

/// Parse la hiérarchie de groupe : "_DSI > _SUPPORT > _PARC" → ["_DSI", "_SUPPORT", "_PARC"]
pub fn parse_groupe_hierarchy(groupe_complet: &str) -> Vec<String> {
    groupe_complet
        .split(" > ")
        .map(|s| {
            s.trim()
                .replace("&amp;", "&")  // Décoder les entités HTML
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Cycle de vie d'un ticket GLPI :
/// Nouveau → En cours (Attribué) → En cours (Planifié) → En attente → Résolu → Clos
///                                                    ↖ reboucle possible ↗
pub fn lifecycle_order(statut: &str) -> u8 {
    match statut {
        "Nouveau"              => 1,
        "En cours (Attribué)"  => 2,
        "En cours (Planifié)"  => 3,
        "En attente"           => 4,
        "Résolu"               => 5,
        "Clos"                 => 6,
        _                      => 0,
    }
}
```

---

## 2. Calculs KPI du stock : âge, distribution et charge

Le module KPI stock analyse l'**inventaire courant des tickets ouverts** (vivants d'un import donné). Ces métriques répondent à la question : « Quel est l'état de santé de notre stock en ce moment ? »

### 2.1 Statistiques d'âge des tickets ouverts

Pour chaque ticket vivant, `anciennete_jours` (pré-calculé lors du parsing CSV, en jours depuis `date_ouverture`) constitue le vecteur d'entrée pour l'analyse statistique. Les quatre métriques fondamentales sont :

- **Moyenne** : tendance générale, sensible aux valeurs extrêmes
- **Médiane** : centre robuste insensible aux outliers (un ticket de 5 ans ne la fausse pas)
- **Écart-type** : dispersion — un écart-type élevé signifie un stock hétérogène (tickets récents + fossiles)
- **P90** : sévérité de la queue — « 90% des tickets sont plus jeunes que X jours »

Les benchmarks MetricNet/HDI pour le support de proximité placent le temps moyen de résolution autour de **8,85 heures ouvrées** pour les incidents. Pour une CPAM avec une charge mixte incidents/demandes, un âge moyen du stock ouvert inférieur à **10 jours ouvrés** est un objectif raisonnable.

#### Fonctions statistiques en Rust pur

Aucun crate externe n'est nécessaire pour ~10K valeurs. Le tri d'un `Vec<f64>` de 10 000 éléments prend ~50µs.

```rust
// src-tauri/src/stats.rs

/// Moyenne arithmétique.
pub fn moyenne(data: &[f64]) -> Option<f64> {
    if data.is_empty() { return None; }
    Some(data.iter().sum::<f64>() / data.len() as f64)
}

/// Médiane (valeur centrale après tri).
pub fn mediane(data: &[f64]) -> Option<f64> {
    if data.is_empty() { return None; }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        Some(sorted[n / 2])
    } else {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

/// Écart-type population (σ, pas l'estimateur σ̂ avec n-1).
/// On utilise la population car on a l'intégralité des tickets, pas un échantillon.
pub fn ecart_type(data: &[f64]) -> Option<f64> {
    let m = moyenne(data)?;
    let variance = data.iter()
        .map(|x| (x - m).powi(2))
        .sum::<f64>() / data.len() as f64;
    Some(variance.sqrt())
}

/// Percentile par interpolation linéaire (identique au défaut NumPy).
/// `p` dans [0.0, 100.0]. Utiliser 90.0 pour P90, 95.0 pour P95.
pub fn percentile(data: &[f64], p: f64) -> Option<f64> {
    if data.is_empty() || !(0.0..=100.0).contains(&p) { return None; }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 1 { return Some(sorted[0]); }
    let rank = (p / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    Some(sorted[lower] * (1.0 - frac) + sorted[upper] * frac)
}

/// Coefficient de variation (CV = σ/μ). Mesure l'homogénéité.
/// CV < 0.2 = très homogène, CV > 0.5 = forte dispersion.
pub fn coefficient_variation(data: &[f64]) -> Option<f64> {
    let m = moyenne(data)?;
    if m == 0.0 { return None; }
    let s = ecart_type(data)?;
    Some(s / m)
}
```

### 2.2 Distribution par tranches d'âge

Les seuils **>30j, >60j, >90j, >180j, >365j** sont alignés avec les pratiques industrielles. BMC Helix ITSM, InvGate et Supportbench utilisent tous les paliers 30/60/90 jours pour les tableaux de bord opérationnels. Les extensions 180 et 365 jours sont des ajouts pragmatiques pour les organisations ayant un historique de backlog — courant dans le secteur public comme la CPAM.

#### Requête SQL en un seul scan

```sql
SELECT
    CASE
        WHEN anciennete_jours < 8    THEN '< 1 sem'
        WHEN anciennete_jours < 30   THEN '1-4 sem'
        WHEN anciennete_jours < 60   THEN '30-60j'
        WHEN anciennete_jours < 90   THEN '60-90j'
        WHEN anciennete_jours < 180  THEN '90-180j'
        WHEN anciennete_jours < 365  THEN '180-365j'
        ELSE '> 1 an'
    END AS tranche_age,
    COUNT(*) AS nb_tickets,
    ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 1) AS pourcentage
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1
GROUP BY tranche_age
ORDER BY
    CASE tranche_age
        WHEN '< 1 sem'  THEN 1  WHEN '1-4 sem'  THEN 2
        WHEN '30-60j'   THEN 3  WHEN '60-90j'    THEN 4
        WHEN '90-180j'  THEN 5  WHEN '180-365j'  THEN 6
        WHEN '> 1 an'   THEN 7
    END;
```

La tranche `< 1 sem` (moins de 8 jours) a été ajoutée car elle capture les tickets frais susceptibles d'être résolus rapidement — une information utile pour le pilotage quotidien.

### 2.3 Tickets sans suivi et inactivité

Les tickets avec `nombre_suivis = 0` sont des « tickets zombies » — créés mais jamais travaillés. C'est un indicateur de qualité critique. Combiné avec `inactivite_jours` (jours depuis `derniere_modification`), il révèle les tickets négligés.

**Données réelles CPAM 92** : sur 543 vivants, **263 n'ont aucun suivi** (48,4%). C'est un signal d'alerte majeur — presque la moitié du stock ouvert n'a jamais été touché.

La bonne pratique est de signaler les tickets avec **zéro suivi ET âgés de plus de 7 jours** comme nécessitant une action immédiate.

```sql
-- Tickets zombies : vivants, sans suivi, âgés de plus de 7 jours
SELECT COUNT(*) AS nb_zombies
FROM tickets
WHERE import_id = ?1
  AND est_vivant = 1
  AND (nombre_suivis IS NULL OR nombre_suivis = 0)
  AND anciennete_jours > 7;
```

### 2.4 Score de charge pondéré par la priorité

Une pondération linéaire simple (Très haute=5, Haute=4, Moyenne=3, Basse=2, Très basse=1) fonctionne mais sous-pondère les tickets critiques. Une **pondération exponentielle** reflète mieux la réalité où un seul P1 peut consommer plus de ressources que cinq P4 :

Le **score de backlog pondéré** est `Σ(tickets_à_priorité_i × poids_i)`. Ce nombre unique permet de suivre si la composition du backlog se dégrade même quand le total reste stable.

```rust
/// Score de charge pondéré d'un ensemble de tickets.
/// Prend en compte la priorité et l'ancienneté pour une mesure composite.
pub fn score_charge_pondere(
    tickets: &[(i64, &str)],  // (anciennete_jours, priorite)
) -> f64 {
    tickets.iter().map(|(age, prio)| {
        let poids = poids_priorite(prio);
        // Facteur d'âge : un ticket vieux pèse plus lourd
        let facteur_age = if *age > 90 { 2.0 }
            else if *age > 30 { 1.5 }
            else { 1.0 };
        poids * facteur_age
    }).sum()
}
```

### 2.5 Analyse de charge par technicien

Les benchmarks HDI sur 1 000 entreprises situent la charge moyenne à **491 tickets résolus par technicien par mois** pour le support de premier niveau. La notion de seuil de stock (nombre maximum de tickets ouverts simultanément) est plus pertinente pour le pilotage quotidien — le CDC définit un seuil par défaut de **20 tickets** par technicien, paramétrable.

L'équilibre de charge au sein de l'équipe se mesure par le **coefficient de variation** (CV = σ/μ des tickets par technicien) : CV < 0,2 indique un bon équilibre, CV > 0,5 révèle un déséquilibre significatif.

```rust
use std::collections::HashMap;

/// Résultat d'analyse de charge pour un technicien.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargeParTechnicien {
    pub nom: String,
    pub nb_vivants: i64,
    pub par_statut: VentilationStatut,
    pub incidents: i64,
    pub demandes: i64,
    pub moyenne_anciennete: f64,
    pub mediane_anciennete: f64,
    pub nb_haute_priorite: i64,      // Haute + Très haute + Majeure
    pub nb_plus_90j: i64,
    pub nb_sans_suivi: i64,
    pub nb_inactifs_14j: i64,
    pub score_charge: f64,
    pub ecart_seuil: i64,            // nb_vivants - seuil (négatif = sous le seuil)
    pub couleur: String,             // "vert", "jaune", "orange", "rouge"
}

/// Ventilation par statut pour un sous-ensemble de tickets.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VentilationStatut {
    pub nouveau: i64,
    pub en_cours_attribue: i64,
    pub en_cours_planifie: i64,
    pub en_attente: i64,
    pub resolu: i64,
    pub clos: i64,
}

impl VentilationStatut {
    pub fn incrementer(&mut self, statut: &str) {
        match statut {
            "Nouveau"              => self.nouveau += 1,
            "En cours (Attribué)"  => self.en_cours_attribue += 1,
            "En cours (Planifié)"  => self.en_cours_planifie += 1,
            "En attente"           => self.en_attente += 1,
            "Résolu"               => self.resolu += 1,
            "Clos"                 => self.clos += 1,
            _                      => {} // Statut inconnu
        }
    }
}

/// Construit l'analyse de charge par technicien en un seul scan SQL + agrégation Rust.
pub fn build_charge_par_technicien(
    conn: &rusqlite::Connection,
    import_id: i64,
    seuil_tickets: i64,
) -> Result<Vec<ChargeParTechnicien>, crate::error::AppError> {
    let mut stmt = conn.prepare_cached(
        "SELECT technicien_principal, statut, type_ticket, priorite,
                anciennete_jours, inactivite_jours, nombre_suivis
         FROM tickets
         WHERE import_id = ?1 AND est_vivant = 1
           AND technicien_principal IS NOT NULL
           AND technicien_principal != ''"
    )?;

    // Accumulateur par technicien
    struct Accum {
        ages: Vec<f64>,
        ventilation: VentilationStatut,
        incidents: i64,
        demandes: i64,
        haute_prio: i64,
        plus_90j: i64,
        sans_suivi: i64,
        inactifs_14j: i64,
        score: f64,
    }

    let mut groups: HashMap<String, Accum> = HashMap::new();
    let mut rows = stmt.query(rusqlite::params![import_id])?;

    while let Some(row) = rows.next()? {
        let tech: String = row.get(0)?;
        let statut: String = row.get(1)?;
        let type_t: String = row.get(2)?;
        let prio: String = row.get(3)?;
        let age: f64 = row.get::<_, Option<i64>>(4)?.unwrap_or(0) as f64;
        let inact: Option<i64> = row.get(5)?;
        let suivis: i64 = row.get::<_, Option<i64>>(6)?.unwrap_or(0);

        let acc = groups.entry(tech).or_insert_with(|| Accum {
            ages: Vec::new(),
            ventilation: VentilationStatut::default(),
            incidents: 0, demandes: 0,
            haute_prio: 0, plus_90j: 0, sans_suivi: 0, inactifs_14j: 0,
            score: 0.0,
        });

        acc.ages.push(age);
        acc.ventilation.incrementer(&statut);
        match type_t.as_str() {
            "Incident" => acc.incidents += 1,
            "Demande"  => acc.demandes += 1,
            _ => {}
        }
        if matches!(prio.as_str(), "Haute" | "Très haute" | "Majeure") {
            acc.haute_prio += 1;
        }
        if age > 90.0 { acc.plus_90j += 1; }
        if suivis == 0 { acc.sans_suivi += 1; }
        if inact.unwrap_or(0) > 14 { acc.inactifs_14j += 1; }

        // Score composite : priorité × facteur d'âge
        let facteur_age = if age > 90.0 { 2.0 } else if age > 30.0 { 1.5 } else { 1.0 };
        acc.score += poids_priorite(&prio) * facteur_age;
    }

    let mut result: Vec<ChargeParTechnicien> = groups.into_iter()
        .map(|(nom, acc)| {
            let nb = acc.ages.len() as i64;
            let moy = crate::stats::moyenne(&acc.ages).unwrap_or(0.0);
            let med = crate::stats::mediane(&acc.ages).unwrap_or(0.0);
            let couleur = couleur_charge(nb, seuil_tickets);

            ChargeParTechnicien {
                nom,
                nb_vivants: nb,
                par_statut: acc.ventilation,
                incidents: acc.incidents,
                demandes: acc.demandes,
                moyenne_anciennete: (moy * 10.0).round() / 10.0,
                mediane_anciennete: med,
                nb_haute_priorite: acc.haute_prio,
                nb_plus_90j: acc.plus_90j,
                nb_sans_suivi: acc.sans_suivi,
                nb_inactifs_14j: acc.inactifs_14j,
                score_charge: (acc.score * 10.0).round() / 10.0,
                ecart_seuil: nb - seuil_tickets,
                couleur,
            }
        })
        .collect();

    // Tri par score de charge décroissant (les plus surchargés en premier)
    result.sort_by(|a, b| b.score_charge.partial_cmp(&a.score_charge)
        .unwrap_or(std::cmp::Ordering::Equal));

    Ok(result)
}
```

---

## 3. Codes couleur et seuils RAG

Le système **RAG (Red/Amber/Green)** est le standard universel ITSM. Pour un public français, le schéma à quatre couleurs vert/jaune/orange/rouge offre une granularité plus fine tout en restant intuitif.

### 3.1 Seuils de charge technicien

Avec un seuil par défaut de **20 tickets** par technicien (paramétrable via la table `config`) :

|Couleur|Condition|Interprétation|
|---|:-:|---|
|**Vert** 🟢|≤ 50% du seuil (≤ 10)|Charge confortable|
|**Jaune** 🟡|51–100% du seuil (11–20)|Charge nominale, à surveiller|
|**Orange** 🟠|101–200% du seuil (21–40)|Surcharge, action nécessaire|
|**Rouge** 🔴|> 200% du seuil (> 40)|Surcharge critique, intervention urgente|

```rust
/// Détermine le code couleur en fonction de la charge et du seuil.
pub fn couleur_charge(nb_vivants: i64, seuil: i64) -> String {
    if seuil == 0 { return "rouge".to_string(); }
    let ratio = nb_vivants as f64 / seuil as f64;
    match ratio {
        r if r <= 0.5 => "vert".to_string(),
        r if r <= 1.0 => "jaune".to_string(),
        r if r <= 2.0 => "orange".to_string(),
        _             => "rouge".to_string(),
    }
}
```

### 3.2 Seuils d'ancienneté des tickets

|Couleur|Ancienneté|Signification|
|---|:-:|---|
|**Vert** 🟢|< 30 jours|Normal, dans la fenêtre de résolution attendue|
|**Jaune** 🟡|30–60 jours|Vieillissant, nécessite un suivi|
|**Orange** 🟠|60–90 jours|Zone de risque, escalade recommandée|
|**Rouge** 🔴|> 90 jours|Backlog critique, action immédiate|

```rust
pub fn couleur_anciennete(jours: i64) -> &'static str {
    match jours {
        0..=29    => "vert",
        30..=59   => "jaune",
        60..=89   => "orange",
        _         => "rouge",
    }
}
```

### 3.3 Indicateur de santé du delta stock

Le **ratio sortie/entrée** est le signal clé : l'objectif est ≥ 1,0, c'est-à-dire que l'équipe résout au moins autant de tickets qu'il en arrive. Un ratio durablement inférieur à 1,0 signifie que le backlog croît indéfiniment.

|Couleur|Delta stock|Interprétation|
|---|:-:|---|
|**Vert** 🟢|≤ 0 (stock en baisse)|Backlog maîtrisé|
|**Jaune** 🟡|+1 à +10%|Croissance modérée, surveiller|
|**Orange** 🟠|+10 à +25%|Croissance significative|
|**Rouge** 🔴|> +25%|Critique : stock en spirale|

---

## 4. Analyse temporelle : le moteur de bilan

Le bilan temporel reconstruit la dynamique des flux de tickets à partir d'un export CSV ponctuel. Puisque l'application ne dispose que d'**une seule photographie** (pas de données time-series continues), le taux de création est dérivé de `date_ouverture` et le taux de clôture est approximé via `date_cloture_approx` (lui-même dérivé de `derniere_modification` pour les tickets terminés).

### 4.1 Approximation de la date de clôture via `derniere_modification`

Le champ `date_cloture_approx` stocke la date de clôture estimée. Le champ `date_mod` interne de GLPI se met à jour à **chaque modification** : changement de statut, ajout de suivi, ajout de tâche, modification de champ, et même les actions automatiques du cron (comme `closeticket` qui fait passer Résolu → Clos après un délai configurable).

**Pour la majorité des tickets terminés, `derniere_modification` est une bonne approximation de la date de clôture**, car la dernière action sur un ticket typique est le changement de statut vers Résolu ou Clos. Toutefois, trois cas dégradent cette approximation :

1. **Modifications post-clôture** : quand un administrateur reclassifie des catégories, lance des mises à jour en masse, ou ajoute des notes administratives aux tickets clos. Chaque action pousse `date_mod` au-delà de la clôture réelle.
    
2. **Délai de clôture automatique** : la `closedate` est fixée quand le cron GLPI s'exécute, pas quand le ticket a été résolu. Si la clôture automatique est configurée à 7 jours après résolution, `date_mod` reflète le moment d'exécution du cron.
    
3. **Cycles de réouverture** (Résolu → En cours → Résolu à nouveau) : chaque cycle met à jour `date_mod`. Le `date_mod` final reflète la dernière résolution, pas la première.
    

**Recommandation** : utiliser `derniere_modification` comme proxy acceptable pour l'analyse de tendance, mais le documenter comme une approximation. Si une précision supérieure est nécessaire, conseiller à l'utilisateur d'ajouter les colonnes `Date de clôture` et `Date de résolution` à sa vue de recherche GLPI avant l'export CSV.

```rust
/// Lors de la normalisation du ticket : attribuer date_cloture_approx
fn date_cloture_approx(statut: &str, derniere_modification: &Option<String>) -> Option<String> {
    if !est_vivant(statut) {
        derniere_modification.clone()
    } else {
        None
    }
}
```

### 4.2 Calcul des flux entrée/sortie par période

**Entrées** (créés) par période : exact pour tous les tickets présents dans l'export — chaque `date_ouverture` est le timestamp de création. **Sorties** (résolus/clos) par période : approximé via `date_cloture_approx` pour les tickets terminés.

**Caveat critique** : les tickets qui ont été créés ET clos avant l'export mais ne figurent pas dans l'extraction sont invisibles. Les taux historiques sous-estiment donc à la fois la création et la clôture. L'approximation s'améliore pour les périodes récentes et se dégrade pour les plus anciennes.

#### Requête SQL — agrégation mensuelle avec delta

```sql
WITH periodes AS (
    -- Union de tous les mois ayant vu une création ou une clôture
    SELECT DISTINCT strftime('%Y-%m', date_ouverture) AS mois
    FROM tickets WHERE import_id = ?1
    UNION
    SELECT DISTINCT strftime('%Y-%m', date_cloture_approx)
    FROM tickets WHERE import_id = ?1 AND date_cloture_approx IS NOT NULL
),
crees AS (
    SELECT strftime('%Y-%m', date_ouverture) AS mois, COUNT(*) AS n
    FROM tickets WHERE import_id = ?1
    GROUP BY mois
),
resolus AS (
    SELECT strftime('%Y-%m', date_cloture_approx) AS mois, COUNT(*) AS n
    FROM tickets WHERE import_id = ?1 AND date_cloture_approx IS NOT NULL
    GROUP BY mois
)
SELECT p.mois,
       COALESCE(c.n, 0) AS nb_crees,
       COALESCE(r.n, 0) AS nb_resolus,
       COALESCE(c.n, 0) - COALESCE(r.n, 0) AS delta
FROM periodes p
LEFT JOIN crees c ON p.mois = c.mois
LEFT JOIN resolus r ON p.mois = r.mois
ORDER BY p.mois;
```

#### Agrégation hebdomadaire (semaines ISO)

SQLite 3.46.0+ supporte `%G` (année ISO semaine) et `%V` (numéro de semaine ISO). **Toujours coupler `%G` avec `%V`** — ne jamais utiliser `%Y` avec `%V`, car aux frontières d'année l'année ISO peut différer de l'année calendaire (ex : le 31 décembre 2024 peut appartenir à 2025-W01).

```sql
-- CORRECT : groupement par semaine ISO
SELECT strftime('%G-S%V', date_ouverture) AS semaine_iso,
       COUNT(*) AS nb_crees
FROM tickets WHERE import_id = ?1
GROUP BY semaine_iso ORDER BY semaine_iso;

-- INCORRECT (erreur fréquente) : mélange année calendaire et semaine ISO
-- strftime('%Y-S%V', date_ouverture)  -- FAUX aux frontières d'année !
```

Pour les versions SQLite antérieures à 3.46.0, le fallback en Rust via chrono est recommandé :

```rust
use chrono::{Datelike, NaiveDateTime};

/// "2026-01-05T16:24:00" → "2026-S02"
pub fn semaine_iso_label(dt: &NaiveDateTime) -> String {
    let iw = dt.date().iso_week();
    format!("{:04}-S{:02}", iw.year(), iw.week())
}

/// "2026-01-05T16:24:00" → "2026-01"
pub fn mois_label(dt: &NaiveDateTime) -> String {
    format!("{:04}-{:02}", dt.year(), dt.month())
}

/// "2026-01-05T16:24:00" → "2026-01-05"
pub fn jour_label(dt: &NaiveDateTime) -> String {
    format!("{}", dt.date())
}
```

**L'agrégation hebdomadaire est la granularité la plus utile** pour le pilotage opérationnel ITSM : elle lisse le bruit quotidien tout en étant assez fine pour détecter les tendances. La granularité mensuelle convient au reporting stratégique/managérial.

### 4.3 Estimation du stock cumulé

La courbe de stock cumulé à la période `t` est `Stock(t) = Stock(t-1) + delta(t)`. Puisque le stock initial est inconnu à partir d'un seul export, on le calcule **à rebours** depuis le stock courant connu : `Stock_actuel = COUNT(est_vivant = 1)`. En remontant les deltas mensuels, on reconstruit la courbe historique.

```rust
/// Calcule le stock cumulé à rebours à partir du stock actuel connu.
/// `rows` doit être trié par période croissante.
/// `stock_actuel` = nombre de tickets vivants dans l'import courant.
pub fn calculer_stock_cumule(
    rows: &mut [BilanTemporelRow],
    stock_actuel: i64,
) {
    let n = rows.len();
    if n == 0 { return; }

    // La dernière période se termine au stock actuel
    rows[n - 1].stock_cumule = Some(stock_actuel);

    // Remonter dans le temps : stock(i) = stock(i+1) - delta(i+1)
    for i in (0..n - 1).rev() {
        let stock_suivant = rows[i + 1].stock_cumule.unwrap_or(0);
        let delta_suivant = rows[i + 1].delta;
        rows[i].stock_cumule = Some(stock_suivant - delta_suivant);
    }
}
```

**Pourquoi à rebours plutôt qu'en avant ?** Le calcul en avant (`Stock(t) = Σ(créés jusqu'à t) - Σ(résolus jusqu'à t)`) ne compte que les tickets visibles dans le snapshot. Les tickets créés et clos avant l'export mais absents de l'extraction sont invisibles → le stock initial serait artificiellement bas. Le calcul à rebours ancre la courbe sur le **stock actuel connu** (fiable car c'est un comptage direct), ce qui produit des estimations historiques plus réalistes.

---

## 5. Structures de données IPC

Tous les types utilisent `#[serde(rename_all = "camelCase")]` pour que le frontend React reçoive des clés JSON en camelCase.

### 5.1 Résultat KPI Stock global

```rust
use serde::Serialize;

/// Vue d'ensemble du stock pour les KPI cards du tableau de bord.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockKpiResult {
    // Comptages globaux
    pub total_vivants: i64,
    pub total_termines: i64,

    // Ventilation par statut
    pub par_statut: Vec<StatutCount>,

    // Statistiques d'âge des vivants
    pub age_moyen_jours: f64,
    pub age_median_jours: f64,
    pub age_ecart_type: f64,
    pub age_p90_jours: f64,
    pub age_p95_jours: f64,

    // Ventilation type
    pub incidents_vivants: i64,
    pub demandes_vivants: i64,

    // Distribution par tranches d'ancienneté
    pub distribution_age: Vec<DistributionAgeBucket>,

    // Indicateurs de qualité
    pub nb_sans_suivi: i64,
    pub pct_sans_suivi: f64,
    pub nb_inactifs_14j: i64,
    pub nb_inactifs_30j: i64,
    pub nb_inactifs_60j: i64,

    // Score de charge global pondéré
    pub score_backlog_pondere: f64,

    // Santé de l'équipe
    pub nb_techniciens_actifs: i64,
    pub charge_moyenne_par_technicien: f64,
    pub cv_charge_techniciens: f64,          // Coefficient de variation
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatutCount {
    pub statut: String,
    pub count: i64,
    pub est_vivant: bool,
    pub pourcentage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionAgeBucket {
    pub label: String,             // "< 1 sem", "1-4 sem", "30-60j", etc.
    pub seuil_min_jours: i64,
    pub seuil_max_jours: Option<i64>,   // None pour la dernière tranche
    pub count: i64,
    pub pourcentage: f64,
    pub couleur: String,           // Code couleur de la tranche
}
```

### 5.2 Résultat Bilan Temporel

```rust
/// Résultat complet du bilan temporel sur une période.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTemporelResult {
    pub granularite: String,        // "jour", "semaine", "mois"
    pub periodes: Vec<BilanTemporelRow>,
    pub totaux: BilanTotaux,
    pub ventilation: Option<Vec<BilanVentilation>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTemporelRow {
    pub periode: String,            // "2026-01", "2026-S05", "2026-01-15"
    pub label: String,              // "Janvier 2026", "Sem. 5", "15/01/2026"
    pub nb_crees: i64,
    pub nb_resolus: i64,
    pub delta: i64,                 // nb_crees - nb_resolus
    pub stock_cumule: Option<i64>,  // Estimé par calcul à rebours
    pub ratio_sortie_entree: Option<f64>,  // nb_resolus / nb_crees
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTotaux {
    pub total_crees: i64,
    pub total_resolus: i64,
    pub delta_global: i64,
    pub moyenne_crees_par_periode: f64,
    pub moyenne_resolus_par_periode: f64,
    pub ratio_global_sortie_entree: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanVentilation {
    pub label: String,              // Nom du technicien/groupe/type
    pub nb_crees: i64,
    pub nb_resolus: i64,
    pub delta: i64,
    pub couleur_delta: String,
}
```

### 5.3 Requête de bilan (entrée frontend → backend)

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanRequest {
    pub granularite: String,         // "jour", "semaine", "mois"
    pub date_debut: Option<String>,  // ISO 8601, None = depuis le début
    pub date_fin: Option<String>,    // ISO 8601, None = jusqu'à maintenant
    pub ventilation_par: Option<String>,  // "technicien", "groupe", "type"
}
```

### 5.4 Types miroir TypeScript

```typescript
// src/types/kpi.ts

export interface StockKpiResult {
  totalVivants: number;
  totalTermines: number;
  parStatut: StatutCount[];
  ageMoyenJours: number;
  ageMedianJours: number;
  ageEcartType: number;
  ageP90Jours: number;
  ageP95Jours: number;
  incidentsVivants: number;
  demandesVivants: number;
  distributionAge: DistributionAgeBucket[];
  nbSansSuivi: number;
  pctSansSuivi: number;
  nbInactifs14j: number;
  nbInactifs30j: number;
  nbInactifs60j: number;
  scoreBacklogPondere: number;
  nbTechniciensActifs: number;
  chargeMoyenneParTechnicien: number;
  cvChargeTechniciens: number;
}

export interface StatutCount {
  statut: string;
  count: number;
  estVivant: boolean;
  pourcentage: number;
}

export interface DistributionAgeBucket {
  label: string;
  seuilMinJours: number;
  seuilMaxJours: number | null;
  count: number;
  pourcentage: number;
  couleur: string;
}

export interface ChargeParTechnicien {
  nom: string;
  nbVivants: number;
  parStatut: VentilationStatut;
  incidents: number;
  demandes: number;
  moyenneAnciennete: number;
  medianeAnciennete: number;
  nbHautePriorite: number;
  nbPlus90j: number;
  nbSansSuivi: number;
  nbInactifs14j: number;
  scoreCharge: number;
  ecartSeuil: number;
  couleur: 'vert' | 'jaune' | 'orange' | 'rouge';
}

export interface VentilationStatut {
  nouveau: number;
  enCoursAttribue: number;
  enCoursPlanifie: number;
  enAttente: number;
  resolu: number;
  clos: number;
}

export interface BilanTemporelResult {
  granularite: 'jour' | 'semaine' | 'mois';
  periodes: BilanTemporelRow[];
  totaux: BilanTotaux;
  ventilation: BilanVentilation[] | null;
}

export interface BilanTemporelRow {
  periode: string;
  label: string;
  nbCrees: number;
  nbResolus: number;
  delta: number;
  stockCumule: number | null;
  ratioSortieEntree: number | null;
}

export interface BilanTotaux {
  totalCrees: number;
  totalResolus: number;
  deltaGlobal: number;
  moyenneCreesParPeriode: number;
  moyenneResolusParPeriode: number;
  ratioGlobalSortieEntree: number;
}

export interface BilanVentilation {
  label: string;
  nbCrees: number;
  nbResolus: number;
  delta: number;
  couleurDelta: string;
}

export interface BilanRequest {
  granularite: 'jour' | 'semaine' | 'mois';
  dateDebut?: string;
  dateFin?: string;
  ventilationPar?: 'technicien' | 'groupe' | 'type';
}
```

---

## 6. Ventilation multi-dimensionnelle : requêtes SQL

### 6.1 Tableau croisé technicien × statut

SQLite n'a pas d'opérateur `PIVOT` natif. On utilise le pattern `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` avec une expression CASE par statut connu :

```sql
SELECT
    technicien_principal,
    SUM(CASE WHEN statut = 'Nouveau' THEN 1 ELSE 0 END) AS nouveau,
    SUM(CASE WHEN statut = 'En cours (Attribué)' THEN 1 ELSE 0 END) AS attribue,
    SUM(CASE WHEN statut = 'En cours (Planifié)' THEN 1 ELSE 0 END) AS planifie,
    SUM(CASE WHEN statut = 'En attente' THEN 1 ELSE 0 END) AS en_attente,
    COUNT(*) AS total,
    ROUND(AVG(anciennete_jours), 1) AS age_moyen,
    SUM(CASE WHEN nombre_suivis = 0 THEN 1 ELSE 0 END) AS sans_suivi,
    SUM(CASE WHEN anciennete_jours > 90 THEN 1 ELSE 0 END) AS plus_90j,
    SUM(CASE WHEN inactivite_jours > 14 THEN 1 ELSE 0 END) AS inactif_14j
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1
GROUP BY technicien_principal
ORDER BY total DESC;
```

### 6.2 Ventilation par groupe hiérarchique

Pour le comptage par groupe le plus spécifique (sans double comptage) :

```sql
SELECT
    COALESCE(groupe_niveau3, groupe_niveau2, groupe_niveau1) AS groupe_effectif,
    groupe_niveau1,
    groupe_niveau2,
    COUNT(*) AS nb_tickets,
    SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
    SUM(CASE WHEN type_ticket = 'Demande' THEN 1 ELSE 0 END) AS demandes,
    COUNT(DISTINCT technicien_principal) AS nb_techniciens,
    ROUND(AVG(anciennete_jours), 1) AS age_moyen
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1
GROUP BY groupe_effectif
ORDER BY nb_tickets DESC;
```

Pour la vue drill-down hiérarchique (avec comptages à chaque niveau) :

```sql
-- Niveau 1 : agrégation la plus large
SELECT
    groupe_niveau1 AS groupe,
    1 AS niveau,
    COUNT(*) AS nb_tickets
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1 AND groupe_niveau1 IS NOT NULL
GROUP BY groupe_niveau1

UNION ALL

-- Niveau 2 : sous-groupes
SELECT
    groupe_niveau1 || ' > ' || groupe_niveau2 AS groupe,
    2 AS niveau,
    COUNT(*) AS nb_tickets
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1 AND groupe_niveau2 IS NOT NULL
GROUP BY groupe_niveau1, groupe_niveau2

ORDER BY niveau, nb_tickets DESC;
```

### 6.3 Gestion des tickets multi-assignés

Un ticket peut être assigné à plusieurs techniciens et/ou plusieurs groupes (champs multilignes séparés par `\n`). La stratégie de comptage dépend du contexte :

|Contexte|Stratégie|Justification|
|---|---|---|
|Stock global|Compter **une fois** via `technicien_principal`|Éviter la surestimation du stock total|
|Vue par technicien|Compter une fois via `technicien_principal`|Le premier technicien est le responsable|
|Vue par groupe|Compter une fois via `groupe_principal`|Idem|
|Analyse de collaboration|Compter **par apparition**|Pour identifier les tickets partagés|
|Export plan d'action|Compter une fois par technicien principal|Un seul responsable par ticket|

Le champ `technicien_principal` (premier de la liste) est le bon axe de ventilation pour le pilotage quotidien. Les techniciens secondaires sont consultés, pas responsables.

### 6.4 Quand agréger en SQL vs en Rust

|Opération|Meilleur dans|Raison|
|---|:-:|---|
|COUNT/SUM/AVG + GROUP BY|**SQL**|Scan unique, pas de transfert|
|Tableaux croisés (CASE WHEN)|**SQL**|Scan unique efficace|
|Groupement temporel (strftime)|**SQL**|Fonctions de date natives|
|Médiane, percentiles|**Rust**|SQLite n'a pas de percentile natif ; trier 10K f64 = ~50µs|
|Scores composites pondérés|**Rust**|Logique multi-champs plus lisible en code|
|Pivots dynamiques (colonnes inconnues)|**Rust construit le SQL**|Requêter d'abord les valeurs DISTINCT, puis construire les colonnes CASE|
|Distribution par tranches d'âge|**Les deux**|CASE SQL ou match Rust — aussi rapides l'un que l'autre|

**Règle de cohérence** : encapsuler toutes les requêtes du tableau de bord dans une seule transaction en lecture pour garantir la cohérence des données :

```rust
pub fn charger_tableau_de_bord(
    conn: &rusqlite::Connection,
    import_id: i64,
) -> Result<TableauDeBord, crate::error::AppError> {
    // Transaction en lecture seule — assure la cohérence entre toutes les requêtes
    let tx = conn.transaction_with_behavior(
        rusqlite::TransactionBehavior::Deferred
    )?;

    let stock_kpi = build_stock_kpi(&tx, import_id)?;
    let charge_techniciens = build_charge_par_technicien(&tx, import_id, 20)?;
    let ventilation_groupes = build_ventilation_groupes(&tx, import_id)?;

    // tx drop sans commit — lecture seule, pas d'effet de bord
    Ok(TableauDeBord { stock_kpi, charge_techniciens, ventilation_groupes })
}
```

Toujours utiliser `prepare_cached()` pour les requêtes du tableau de bord — ça réutilise les statements compilés et élimine le surcoût de préparation lors des rafraîchissements successifs.

---

## 7. Commandes Tauri et intégration IPC

### 7.1 Commande KPI Stock

```rust
// src-tauri/src/commands/stock.rs
use tauri::State;
use crate::state::{AppState, DbAccess};

#[tauri::command]
pub async fn get_stock_kpi(
    state: State<'_, AppState>,
) -> Result<StockKpiResult, String> {
    state.db(|conn| {
        let import_id = get_active_import_id(conn)?;
        build_stock_kpi(conn, import_id)
    })
}

#[tauri::command]
pub async fn get_charge_techniciens(
    state: State<'_, AppState>,
) -> Result<Vec<ChargeParTechnicien>, String> {
    state.db(|conn| {
        let import_id = get_active_import_id(conn)?;
        let seuil = get_config_value(conn, "seuil_tickets_technicien")?
            .parse::<i64>()
            .unwrap_or(20);
        build_charge_par_technicien(conn, import_id, seuil)
    })
}

/// Récupère l'ID de l'import actif.
fn get_active_import_id(conn: &rusqlite::Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT id FROM imports WHERE is_active = 1 ORDER BY id DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
}

/// Récupère une valeur de configuration.
fn get_config_value(conn: &rusqlite::Connection, key: &str) -> Result<String, rusqlite::Error> {
    conn.query_row(
        "SELECT value FROM config WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
}
```

### 7.2 Commande Bilan Temporel

```rust
// src-tauri/src/commands/bilan.rs

#[tauri::command]
pub async fn get_bilan_temporel(
    state: State<'_, AppState>,
    request: BilanRequest,
) -> Result<BilanTemporelResult, String> {
    state.db(|conn| {
        let import_id = get_active_import_id(conn)?;
        build_bilan_temporel(conn, import_id, &request)
    })
}
```

### 7.3 Appel depuis le frontend

```typescript
// src/hooks/useStockKpi.ts
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { StockKpiResult } from '../types/kpi';
import { useAppStore } from '../stores/appStore';

export function useStockKpi() {
  const [data, setData] = useState<StockKpiResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeImportId = useAppStore((s) => s.activeImportId);

  useEffect(() => {
    if (!activeImportId) return;

    setLoading(true);
    invoke<StockKpiResult>('get_stock_kpi')
      .then(setData)
      .catch((err) => setError(typeof err === 'string' ? err : String(err)))
      .finally(() => setLoading(false));
  }, [activeImportId]);

  return { data, loading, error };
}

// src/hooks/useBilanTemporel.ts
import { invoke } from '@tauri-apps/api/core';
import type { BilanTemporelResult, BilanRequest } from '../types/kpi';

export async function fetchBilanTemporel(
  request: BilanRequest
): Promise<BilanTemporelResult> {
  return invoke<BilanTemporelResult>('get_bilan_temporel', { request });
}
```

---

## 8. Indicateurs ITIL complémentaires

Au-delà du stock et des flux, plusieurs métriques ITIL peuvent être calculées à partir du même jeu de données :

### 8.1 Délai moyen de résolution (MTTR)

Le **Mean Time To Resolve** se calcule comme la moyenne de `date_cloture_approx - date_ouverture` pour les tickets résolus.

```sql
SELECT
    ROUND(AVG(
        julianday(date_cloture_approx) - julianday(date_ouverture)
    ), 1) AS mttr_jours,
    -- Par type
    ROUND(AVG(CASE WHEN type_ticket = 'Incident'
        THEN julianday(date_cloture_approx) - julianday(date_ouverture)
    END), 1) AS mttr_incidents,
    ROUND(AVG(CASE WHEN type_ticket = 'Demande'
        THEN julianday(date_cloture_approx) - julianday(date_ouverture)
    END), 1) AS mttr_demandes
FROM tickets
WHERE import_id = ?1
  AND est_vivant = 0
  AND date_cloture_approx IS NOT NULL;
```

### 8.2 Taux de résolution au premier contact

Approximé par les tickets résolus avec `nombre_suivis ≤ 1`. L'objectif ITIL typique est **70–75%**.

```sql
SELECT
    COUNT(*) AS total_resolus,
    SUM(CASE WHEN nombre_suivis <= 1 THEN 1 ELSE 0 END) AS premier_contact,
    ROUND(100.0 * SUM(CASE WHEN nombre_suivis <= 1 THEN 1 ELSE 0 END)
        / NULLIF(COUNT(*), 0), 1) AS taux_premier_contact
FROM tickets
WHERE import_id = ?1 AND est_vivant = 0;
```

### 8.3 Distribution des délais de résolution

Par tranches temporelles pour évaluer la performance SLA :

```sql
SELECT
    CASE
        WHEN julianday(date_cloture_approx) - julianday(date_ouverture) <= 1 THEN '≤ 1 jour'
        WHEN julianday(date_cloture_approx) - julianday(date_ouverture) <= 7 THEN '2-7 jours'
        WHEN julianday(date_cloture_approx) - julianday(date_ouverture) <= 30 THEN '8-30 jours'
        WHEN julianday(date_cloture_approx) - julianday(date_ouverture) <= 90 THEN '31-90 jours'
        ELSE '> 90 jours'
    END AS tranche_delai,
    COUNT(*) AS nb_tickets,
    ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 1) AS pourcentage
FROM tickets
WHERE import_id = ?1
  AND est_vivant = 0
  AND date_cloture_approx IS NOT NULL
GROUP BY tranche_delai
ORDER BY
    CASE tranche_delai
        WHEN '≤ 1 jour'    THEN 1
        WHEN '2-7 jours'   THEN 2
        WHEN '8-30 jours'  THEN 3
        WHEN '31-90 jours' THEN 4
        WHEN '> 90 jours'  THEN 5
    END;
```

### 8.4 Comparatif inter-techniciens (bilan d'activité)

```sql
SELECT
    technicien_principal,
    COUNT(*) AS nb_resolus,
    ROUND(AVG(julianday(date_cloture_approx) - julianday(date_ouverture)), 1) AS delai_moyen_jours,
    ROUND(AVG(COALESCE(nombre_suivis, 0)), 1) AS suivis_moyen,
    SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
    SUM(CASE WHEN type_ticket = 'Demande' THEN 1 ELSE 0 END) AS demandes,
    SUM(CASE WHEN nombre_suivis <= 1 THEN 1 ELSE 0 END) AS premier_contact
FROM tickets
WHERE import_id = ?1 AND est_vivant = 0
  AND technicien_principal IS NOT NULL
  AND technicien_principal != ''
GROUP BY technicien_principal
ORDER BY nb_resolus DESC;
```

---

## 9. Structure recommandée des modules

```
src-tauri/src/
├── stats.rs              // moyenne, mediane, ecart_type, percentile, coefficient_variation
├── date_utils.rs         // semaine_iso_label, mois_label, jour_label,
│                         //   couleur_anciennete, parse_datetime
├── classification.rs     // est_vivant, poids_priorite, couleur_charge,
│                         //   parse_groupe_hierarchy, lifecycle_order
├── analyzer/
│   ├── mod.rs
│   ├── stock.rs          // build_stock_kpi(), build_charge_par_technicien()
│   ├── bilan.rs          // build_bilan_temporel(), calculer_stock_cumule()
│   ├── ventilation.rs    // ventilation_par_statut(), ventilation_par_groupe(),
│   │                     //   pivot_technicien_statut()
│   └── delais.rs         // mttr(), distribution_delais(), taux_premier_contact()
├── commands/
│   ├── stock.rs          // #[tauri::command] get_stock_kpi, get_charge_techniciens, etc.
│   └── bilan.rs          // #[tauri::command] get_bilan_temporel
└── models/
    ├── stock.rs           // StockKpiResult, ChargeParTechnicien, VentilationStatut, etc.
    └── bilan.rs           // BilanTemporelResult, BilanTemporelRow, BilanTotaux, etc.
```

---

## 10. Récapitulatif des décisions d'architecture

|Décision|Choix|Justification|
|---|---|---|
|Classification vivant/terminé|Négation : `!matches!(statut, "Clos" \| "Résolu")`|Tout statut inconnu = vivant (principe de précaution)|
|Calculs statistiques|Rust pur (`Vec<f64>` + tri)|Pas de crate externe, ~50µs pour 10K valeurs|
|Agrégations comptables|SQL (GROUP BY + CASE WHEN)|Scan unique, pas de transfert de données|
|Date de clôture|Proxy via `derniere_modification`|Acceptable pour l'analyse de tendance, documenté comme approximation|
|Stock cumulé historique|Calcul à rebours depuis stock connu|Plus fiable que le calcul en avant (snapshot incomplet)|
|Semaine ISO|`strftime('%G-S%V')` SQLite 3.46+ ou `chrono::IsoWeek` Rust|`%Y` + `%V` = bug aux frontières d'année|
|Seuils couleur|4 niveaux RAG (vert/jaune/orange/rouge)|Standard ITSM, paramétrable via table `config`|
|Pondération priorité|Exponentielle (Majeure=10, Très haute=8, Haute=5, Moyenne=3)|Un P1 ≠ cinq P4 en effort réel|
|Tickets multi-assignés|`technicien_principal` (1er de la liste)|Un seul responsable par ticket pour le pilotage|
|Cohérence des requêtes|Transaction en lecture pour le tableau de bord|Données cohérentes entre toutes les KPI cards|
|Priorité « Majeure »|Acceptée avec poids maximal|Présente dans les données réelles CPAM 92|

---

_Ce segment fournit l'intégralité de la logique de calcul pour les modules Stock et Bilan du GLPI Dashboard. Il s'appuie sur les structures de données du Segment 2 (SQLite) et consomme les données parsées par le Segment 1 (CSV). Le Segment 4 (catégories hiérarchiques) exploitera les mêmes patterns de ventilation en ajoutant la dimension catégorielle._