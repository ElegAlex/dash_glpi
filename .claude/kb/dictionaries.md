# Dictionnaires Mots-Clés — GLPI Dashboard

Source : Segments 2 & 5

---

## Dictionnaires de Classification (table keyword_dictionaries)

### Catégorie `resolution`
Mots indiquant qu'un ticket est résolu implicitement :

```sql
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('resolution', 'résolu'),
    ('resolution', 'réglé'),
    ('resolution', 'terminé'),
    ('resolution', 'effectué'),
    ('resolution', 'remplacé'),
    ('resolution', 'installé'),
    ('resolution', 'livré'),
    ('resolution', 'configuré'),
    ('resolution', 'fonctionnel'),
    ('resolution', 'c''est bon'),
    ('resolution', 're-fonctionne'),
    ('resolution', 'refonctionne'),
    ('resolution', 'opérationnel'),
    ('resolution', 'corrigé'),
    ('resolution', 'mis à jour'),
    ('resolution', 'déployé'),
    ('resolution', 'activé'),
    ('resolution', 'débloqu');
```

### Catégorie `relance`
Mots indiquant qu'un ticket nécessite une relance :

```sql
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('relance', 'toujours d''actualité'),
    ('relance', 'impossibilité de vous joindre'),
    ('relance', 'sans nouvelles'),
    ('relance', 'relance'),
    ('relance', 'en attente de retour'),
    ('relance', 'merci de confirmer');
```

### Catégorie `annulation`
Mots indiquant qu'un ticket est annulé/doublon :

```sql
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('annulation', 'annulé'),
    ('annulation', 'doublon'),
    ('annulation', 'obsolète'),
    ('annulation', 'plus d''actualité'),
    ('annulation', 'ne plus traiter');
```

### Catégorie `exclusion`
Phrases boilerplate GLPI à ignorer lors de l'analyse :

```sql
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('exclusion', 'pièce jointe liée lors de la création'),
    ('exclusion', 'ticket créé automatiquement'),
    ('exclusion', 'mail collecteur');
```

---

## Stop Words NLP (Segment 5)

Architecture en 4 couches — traiter dans cet ordre :

### Couche 3+4 : Phrases templates GLPI (Aho-Corasick, AVANT tokenisation)

Environ 25 phrases :
- `"Suite contact avec"`
- `"L'incident ou la demande"`
- `"Les actions suivantes ont été effectuées"`
- `"Pièce jointe liée lors de la création"`
- `"Assigné au groupe"`
- `"Assigné au technicien"`
- `"Changement de statut"`
- `"Nouveau ticket créé"`
- `"Ticket résolu"`
- `"Ticket clos"`
- `"En attente de"`
- `"Validation demandée à"`
- `"Suivi ajouté par"`
- `"Tâche ajoutée par"`
- `"Solution ajoutée par"`
- `"Mise à jour du ticket"`
- `"Ce ticket a été créé automatiquement"`
- `"Merci de ne pas répondre à ce message"`
- `"Satisfaction demandée le"`
- `"Planifié le"`

### Couche 4 : Signatures dynamiques (Aho-Corasick)
- `"Intervenant équipe Support"`
- `"Intervenant équipe Réseau"`
- `"Service desk"`
- `"Support technique"`
- `"Direction des Systèmes d'Information"`
- `"Envoyé depuis"`
- `"Ce message et ses pièces jointes"`
- `"N'imprimez ce mail que si nécessaire"`
- + noms des techniciens (chargés depuis la DB)

### Couche 1 : Français standard (~160 mots, HashSet, APRÈS tokenisation)

Articles : `le`, `la`, `les`, `un`, `une`, `des`, `du`

Prépositions : `à`, `au`, `dans`, `sur`, `pour`, `avec`, `sans`, `entre`, `vers`

Pronoms : `je`, `tu`, `il`, `elle`, `nous`, `vous`, `ils`, `on`, `ce`, `cette`

Conjonctions : `et`, `ou`, `mais`, `donc`, `car`, `que`, `qui`

Auxiliaires : `est`, `sont`, `a`, `ai`, `ont`, `était`, `été`, `eu`

Résidus d'élision : `l`, `d`, `n`, `s`, `c`, `qu`, `j`

### Couche 2 : Domaine IT/ITSM (~50 mots, HashSet)

Formules politesse : `bonjour`, `cordialement`, `cdlt`, `salutations`, `merci`, `svp`, `veuillez`

Fillers ticket : `objet`, `ref`, `référence`, `ci-joint`, `ci-dessous`, `pj`

Accusés réception : `reçu`, `enregistré`, `noté`, `transmis`

Mots ITSM faible signal : `ticket`, `incident`, `demande`, `numéro`, `dossier`, `urgent`, `priorité`, `date`

---

## Implémentation Aho-Corasick (Segment 5)

```rust
use aho_corasick::AhoCorasick;
use std::collections::HashSet;

pub struct StopWordFilter {
    phrase_automaton: AhoCorasick,   // Couches 3+4
    pattern_count: usize,
    single_words: HashSet<String>,   // Couches 1+2
}

impl StopWordFilter {
    pub fn remove_phrases(&self, text: &str) -> String {
        let replacements = vec![""; self.pattern_count];
        self.phrase_automaton.replace_all(text, &replacements)
    }

    pub fn is_stop_word(&self, token: &str) -> bool {
        self.single_words.contains(token)
    }

    pub fn add_technician_names(&mut self, names: &[String]) {
        for name in names {
            for part in name.split_whitespace() {
                self.single_words.insert(part.to_lowercase());
            }
        }
    }
}
```

Configuration Aho-Corasick : `ascii_case_insensitive(true)` pour les phrases templates.

---

## Pipeline NLP complet

```
Texte brut
    │
    ▼ Nettoyage HTML (strip_html via regex)
    │
    ▼ Couche 3+4 : Aho-Corasick sur phrases GLPI + signatures
    │
    ▼ Tokenisation Charabia (NFKD + accents → lowercase, is_word() uniquement)
    │
    ▼ Filtre tokens len > 1 (exclure résidus élision : l, d, n)
    │
    ▼ Couche 1+2 : HashSet stop words
    │
    ▼ Stemming Snowball français (rust-stemmers v1.2.0)
    │
    Tokens propres → TF-IDF
```

---

## Versions Crates NLP

| Crate | Version | Usage |
|---|---|---|
| `charabia` | 0.9.9 | Tokenisation française (NFKD + accents) |
| `rust-stemmers` | 1.2.0 | Stemming Snowball français |
| `sprs` | 0.11.4 | Matrices creuses TF-IDF (CsMat/TriMat) |
| `aho-corasick` | 1.x | Suppression phrases templates GLPI |
| `regex` | 1.x | Strip HTML, tokenizer alternatif |

Configuration Cargo :
```toml
charabia = { version = "0.9", default-features = false }  # Désactive jieba/lindera (~20MB)
rust-stemmers = "1.2"
sprs = "0.11"
aho-corasick = "1"
```
