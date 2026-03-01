# Règles Métier — GLPI Dashboard

Sources : Segments 1-8

---

## Format CSV (Segments 1)

**RG-001 : Format CSV**
- Séparateur : point-virgule (`;`)
- Encodage : UTF-8 avec BOM (géré nativement par csv 1.4.0)
- Format dates : `DD-MM-YYYY HH:MM` (ex: `05-01-2026 16:24`)
- Quoting : RFC 4180 (guillemets doubles, `""` pour échapper)

**RG-002 : IDs avec espaces**
- Les IDs GLPI contiennent des espaces insécables (`5 732 943`)
- Nettoyage : supprimer tous les caractères whitespace → `5732943`
- Type Rust : `u64`
- Désérialiseur : `de::spaced_u64`

**RG-003 : Champs multilignes**
- Séparateur : `\n` (retour à la ligne réel entre guillemets)
- Champs concernés : `Attribué à - Technicien`, `Attribué à - Groupe de techniciens`, `Suivis - Description`, `Solution - Solution`, `Tâches - Description`
- Gestion native par le crate csv (RFC 4180 : `\n` en dehors des guillemets = fin de record)

**RG-004 : Colonnes obligatoires vs optionnelles**
- **Obligatoires** : `ID`, `Titre`, `Statut`, `Date d'ouverture`, `Type`
- **Optionnelles** : `Catégorie` (absente du CSV actuel, future)
- Si colonne obligatoire manquante → erreur `CsvImportError::MissingColumns`
- Si colonne optionnelle manquante → warning, valeur `None`

---

## Classification Vivant/Terminé (Segment 3)

**RG-005 : Classification statuts**

GLPI 9.5 définit exactement 6 statuts :

| Code | Constante PHP | Libellé CSV français | Classification |
|:---:|---|---|:---:|
| 1 | `INCOMING` | `Nouveau` | **Vivant** |
| 2 | `ASSIGNED` | `En cours (Attribué)` | **Vivant** |
| 3 | `PLANNED` | `En cours (Planifié)` | **Vivant** |
| 4 | `WAITING` | `En attente` | **Vivant** |
| 5 | `SOLVED` | `Résolu` | **Terminé** |
| 6 | `CLOSED` | `Clos` | **Terminé** |

Champ DB : `est_vivant INTEGER` (1 = vivant, 0 = terminé)

---

## Priorités (Segment 3)

**RG-006 : Poids priorité**

| Libellé | Poids |
|---|:---:|
| Très haute | 5 |
| Haute | 4 |
| Majeure | 4 |
| Moyenne | 3 |
| Basse | 2 |
| Très basse | 1 |

---

## Seuils RAG Charge Technicien (Segments 2 & 3)

**RG-007 : Seuils RAG charge technicien**
- Seuil configurable (défaut : 20 tickets)
- Vert : `< 50%` du seuil (< 10)
- Jaune : `50-100%` du seuil (10-20)
- Orange : `100-200%` du seuil (20-40)
- Rouge : `> 200%` du seuil (> 40)

Valeurs config DB :
- `seuil_couleur_vert` = 10
- `seuil_couleur_jaune` = 20
- `seuil_couleur_orange` = 40
- `seuil_tickets_technicien` = 20

Champ DB `couleur_seuil` : `"vert"`, `"jaune"`, `"orange"`, `"rouge"`

---

## Seuils Ancienneté/Inactivité (Segment 2)

**RG-008 : Seuil fermeture**
- Ancienneté > 90 jours → recommandation clôture
- Config : `seuil_anciennete_cloturer` = 90

**RG-009 : Seuil inactivité**
- Inactivité > 14 jours → recommandation relance
- Config : `seuil_inactivite_relancer` = 14

**RG-008b : Seuil inactivité clôture**
- Inactivité > 60 jours → recommandation clôture
- Config : `seuil_inactivite_cloturer` = 60

**RG-008c : Seuil ancienneté relance**
- Ancienneté > 30 jours → recommandation relance
- Config : `seuil_anciennete_relancer` = 30

---

## Tickets Zombies (Segment 3)

**RG-010 : Détection zombie**
- Ticket vivant avec 0 suivis (`nombre_suivis = 0` ou `NULL`)
- Action recommandée : `'qualifier'` (à qualifier/clôturer)

---

## Hiérarchie Groupes (Segment 4)

**RG-011 : Parsing hiérarchie groupes**
- Séparateur niveau : ` > ` (espace-chevron-espace)
- Décodage HTML entities (ex: `&amp;` → `&`)
- Maximum 3 niveaux (groupe_niveau1, groupe_niveau2, groupe_niveau3)
- Exemple : `_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL > _SUPPORT - PARC`
  - niveau1 = `_DSI`
  - niveau2 = `_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`
  - niveau3 = `_SUPPORT - PARC`

**Données réelles CPAM 92** (9 616 tickets, 8 groupes distincts) :
| Groupe complet | Tickets | % |
|---|--:|--:|
| `_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL` | 6 562 | 68,2% |
| `_DSI > _PRODUCTION-INFRASTRUCTURES` | 1 474 | 15,3% |
| `_DSI > _SERVICE DES CORRESPONDANTS INFORMATIQUE` | 1 178 | 12,3% |
| `_DSI > _HABILITATIONS_PRODUCTION` | 302 | 3,1% |
| `_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL > _SUPPORT - PARC` | 165 | 1,7% |
| `_DSI > _DIADEME` | 31 | 0,3% |

---

## Technicien/Groupe Principal (Segment 1)

**RG-012 : Technicien principal**
- = premier élément de la liste multiligne `Attribué à - Technicien`
- Split sur `\n`, trim, filter non-empty, prendre `first()`

**RG-013 : Groupe principal**
- = premier élément de la liste multiligne `Attribué à - Groupe de techniciens`
- Split sur `\n`, trim, filter non-empty, prendre `first()`

---

## Calcul Ancienneté (Segment 1)

**RG-014 : Ancienneté**
- Ticket vivant : `now - date_ouverture` en jours
- Ticket terminé : `derniere_modification - date_ouverture` en jours
- Implémentation : `chrono::Utc::now().naive_utc()`
- Format date ISO 8601 : `%Y-%m-%dT%H:%M:%S`

**RG-015 : Inactivité**
- = `now - derniere_modification` en jours
- Uniquement si `derniere_modification` est non-null

---

## NLP / Text Mining (Segment 5)

**RG-016 : Seuil de similarité doublons**
- Seuil TF-IDF similarité cosinus : 0.65
- En dessous : tickets distincts
- Au-dessus : suspects doublons

**RG-017 : Normalisation texte**
- Pipeline : nettoyage HTML → suppression phrases GLPI (Aho-Corasick) → tokenisation Charabia → stop words 4 couches → stemming Snowball français
- Charabia v0.9.9, `default-features = false` (désactive jieba/lindera ~20MB)
- rust-stemmers v1.2.0, `Algorithm::French`

**RG-018 : Paramètres TF-IDF**
- `sublinear_tf = true` (1 + log(tf) — essentiel pour documents courts)
- `smooth_idf = true` (log((1+N)/(1+df)) + 1)
- `l2_normalize = true` (normalisation L2 des vecteurs)
- `min_df = 2` (exclure les hapax/typos)
- `max_df_ratio = 0.90` (exclure les mots trop communs)

---

## Clustering (Segment 6)

**RG-019 : K-Means**
- Crate : `linfa-clustering v0.8.1`
- Nombre de clusters : 5-15 (auto-sélection via silhouette)
- Input : matrice TF-IDF dense (conversion CsMat → Array2<f64>)
- Contrainte mémoire : 10K×5K × f32 = ~200 MB OK sur desktop

**RG-020 : Anomalies**
- Z-scores sur délais log-transformés
- Seuil anomalie : |z-score| > 2.5
- Types : `"delai_anormal"`, `"categorie_inhabituelle"`, `"dormant"`

---

## Exports (Segment 8)

**RG-021 : Format dates Excel**
- Format français : `"dd/mm/yyyy"` (Excel traduit selon locale Windows)
- Séparateur milliers : `"#,##0"` (espace en locale française)
- Décimales : `"#,##0.00"` (virgule en locale française)
- Euro : `"#,##0.00 [$€-fr-FR]"`

**RG-022 : Couleurs seuils Excel**
- Vert (0-10) : bg `C6EFCE`, font `006100`
- Jaune (11-20) : bg `FFEB9C`, font `9C6500`
- Orange (21-40) : bg `F4B084`, font `833C0C`
- Rouge (41+) : bg `FFC7CE`, font `9C0006`

**RG-023 : Exports disponibles**
- Tableau de bord stock (3 onglets : Vue globale + Techniciens + Groupes)
- Plan d'action individuel (3 onglets : Entretien + Détail tickets + Checklist)
- Bilan d'activité (3 onglets : Volume + Délais + Comparatif techniciens)
- Rapport de suivi (2 onglets : Évolution stock + Delta techniciens)
