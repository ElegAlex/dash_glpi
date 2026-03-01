# EP05 — NLP et text mining

## Description

Le module NLP extrait le signal sémantique des champs texte des tickets (titres, suivis, solutions, tâches) en français. Il exécute un pipeline complet tokenisation Charabia → filtrage stop words 4 couches → stemming Snowball → vectorisation TF-IDF creuse (sprs) en < 100 ms pour 10 000 tickets. Le frontend expose un nuage de mots interactif (@visx/wordcloud) et une recherche full-text via SQLite FTS5. Les mots-clés sont calculables par groupe de techniciens ou pour tout le corpus.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-041 | La tokenisation utilise Charabia v0.9 (`default-features = false`) pour le français |
| RG-042 | Le stemming utilise Snowball français (rust-stemmers v1.2) |
| RG-043 | Les stop words s'appliquent en 4 couches : français standard, IT/ITSM, templates GLPI, signatures |
| RG-044 | TF-IDF avec `sublinear_tf = true`, `smooth_idf = true`, `l2_normalize = true` |
| RG-045 | Le filtre `min_df = 2` exclut les hapax/fautes de frappe |
| RG-046 | La matrice TF-IDF est mise en cache (`Arc<TfIdfResult>`) et recalculée après chaque import |
| RG-047 | La recherche full-text utilise SQLite FTS5 avec `tokenize='unicode61 remove_diacritics 2'` |
| RG-048 | Les mots-clés par groupe sont calculés par agrégation des scores TF-IDF des documents du groupe |

## User stories

### US016 — Extraction mots-clés TF-IDF globaux

**Module cible :** `src-tauri/src/nlp/pipeline.rs`, `src-tauri/src/nlp/tfidf.rs`, `src-tauri/src/commands/mining.rs`

**GIVEN** un corpus de tickets a été importé et le pipeline NLP a été exécuté
**WHEN** la commande `run_text_analysis` est invoquée
**THEN** les 20 mots-clés globaux les plus représentatifs (score TF-IDF agrégé sur tout le corpus) sont retournés avec leur score, les termes boilerplate GLPI filtrés, et les statistiques du corpus (n_docs, n_vocab, sparsité)

**Critères de validation :**
- [ ] Le pipeline complet (tokenisation → TF-IDF) s'exécute en < 100 ms pour 10 000 tickets (RG-046)
- [ ] Les phrases templates GLPI (`"Suite contact avec"`, `"Assigné au groupe"`) sont absentes des résultats (RG-043)
- [ ] Les mots `ticket`, `incident`, `demande`, `urgent` sont exclus (stop words ITSM, RG-043)
- [ ] Les résidus d'élision (`l`, `d`, `n`) sont filtrés (Charabia + stop words, RG-041)
- [ ] Les statistiques corpus sont incluses dans la réponse (`CorpusStats`)

---

### US017 — Recherche full-text FTS5

**Module cible :** `src-tauri/src/commands/mining.rs` (ou `commands/search.rs`), `src/pages/StockDashboard.tsx`

**GIVEN** l'index FTS5 SQLite est alimenté lors de chaque import
**WHEN** l'utilisateur saisit un terme de recherche dans la barre de recherche
**THEN** les tickets correspondants sont retournés en < 50 ms, avec support des requêtes booléennes (`AND`, `OR`, `NOT`), des préfixes (`réseau*`), et insensibilité aux accents (`reseau` trouve `réseau`)

**Critères de validation :**
- [ ] La recherche `"réseau"` trouve les tickets contenant "réseau" (avec ou sans accent) (RG-047)
- [ ] La recherche `"imprimante AND driver"` retourne uniquement les tickets contenant les deux termes
- [ ] La recherche `"réseau*"` trouve "réseau", "réseau_local", "réseaux"
- [ ] Les résultats sont classés par pertinence BM25
- [ ] Les résultats s'affichent en < 50 ms pour un corpus de 10 000 tickets

---

### US018 — Nuage de mots interactif

**Module cible :** `src/components/WordCloud.tsx` (@visx/wordcloud), `src/pages/StockDashboard.tsx`

**GIVEN** les mots-clés TF-IDF ont été calculés pour le corpus ou un groupe
**WHEN** la section nuage de mots est affichée
**THEN** les 50 mots-clés les plus significatifs sont rendus en SVG via @visx/wordcloud avec une taille proportionnelle au score TF-IDF (échelle logarithmique), et un clic sur un mot filtre le tableau de tickets sur ce terme

**Critères de validation :**
- [ ] La taille des mots suit une échelle logarithmique (`scaleLog` de @visx/scale) (RG-044)
- [ ] Le layout est horizontal (`rotate={0}`) pour la lisibilité du français
- [ ] Le layout est déterministe (`random={() => 0.5}`)
- [ ] Cliquer sur un mot déclenche une recherche FTS5 et filtre le tableau de tickets
- [ ] Le survol d'un mot met en évidence celui-ci (opacité 0.4 sur les autres)

---

### US019 — Mots-clés par groupe de techniciens

**Module cible :** `src-tauri/src/nlp/tfidf.rs` (fn `top_keywords_for_group`), `src-tauri/src/commands/mining.rs`, `src/pages/TechnicianDetail.tsx`

**GIVEN** les scores TF-IDF sont calculés pour tout le corpus
**WHEN** l'utilisateur consulte le détail d'un groupe ou d'un technicien
**THEN** les 10 mots-clés spécifiques au groupe/technicien sont calculés par agrégation des scores TF-IDF des documents du groupe, et affichés dans un mini-nuage de mots ou une liste ordonnée

**Critères de validation :**
- [ ] Les mots-clés d'un groupe sont distincts des mots-clés globaux (terme spécifique au groupe) (RG-048)
- [ ] Le calcul est instantané (agrégation sur la matrice en cache, pas de recalcul)
- [ ] Les noms de techniciens du groupe sont filtrés (stop words dynamiques, RG-043)
- [ ] Les résultats sont cohérents : le groupe `_PRODUCTION` devrait avoir des mots-clés différents du groupe `_SUPPORT`

## Critères de succès de l'epic

- [ ] Pipeline NLP < 100 ms pour 10 000 tickets (single-thread, RG-046)
- [ ] La recherche FTS5 retourne des résultats pertinents insensibles aux accents (RG-047)
- [ ] Le nuage de mots est interactif et déclenche des filtres cohérents avec le tableau tickets
- [ ] Les mots-clés par groupe produisent des résultats distincts entre groupes différents
