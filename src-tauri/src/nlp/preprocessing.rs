//! NLP Preprocessing — tokenizer, stop words FR + ITSM, stemmer
//!
//! Full pipeline per ticket text:
//!   HTML removal → template-phrase removal → charabia tokenisation
//!   → length filter → stop-word filter → Snowball-FR stemming

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use charabia::Tokenize;
use regex::Regex;
use rust_stemmers::{Algorithm, Stemmer};

// ── Static regex ──────────────────────────────────────────────────────────────

static HTML_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)<[^>]*>|&amp;|&lt;|&gt;|&quot;|&apos;|&nbsp;|&#\d+;|&#x[0-9a-fA-F]+;",
    )
    .expect("HTML_REGEX: invalid pattern")
});

// ── Stop-word constants ───────────────────────────────────────────────────────

/// Layer 1 — Standard French stop words (~160 words)
const FRENCH_STOP_WORDS: &[&str] = &[
    // Articles
    "le", "la", "les", "un", "une", "des", "du", "de", "au", "aux",
    // Prépositions
    "a", "à", "dans", "sur", "pour", "avec", "sans", "entre", "vers", "par", "en",
    "chez", "contre", "sous", "devant", "derrière", "après", "avant", "pendant",
    "depuis", "dès", "jusqu",
    // Pronoms personnels
    "je", "tu", "il", "elle", "nous", "vous", "ils", "elles", "on",
    // Pronoms démonstratifs / relatifs
    "ce", "cette", "ces", "celui", "celle", "ceux", "celles",
    "se", "me", "te", "lui", "leur", "y", "moi", "toi", "soi",
    "qui", "que", "quoi", "dont", "où", "lequel", "laquelle", "lesquels", "lesquelles",
    // Conjonctions
    "et", "ou", "mais", "donc", "car", "ni", "si", "comme",
    "lorsque", "quand", "puisque", "parce", "pourtant", "cependant", "or", "tandis",
    // Auxiliaires / verbes communs
    "est", "sont", "ai", "ont", "était", "été", "eu",
    "fait", "faire", "être", "avoir", "peut", "doit", "va", "vont",
    "sera", "seront", "avait", "avaient", "serait", "soit", "faut",
    // Adverbes
    "ne", "pas", "plus", "très", "bien", "aussi", "encore", "même",
    "tout", "tous", "toute", "toutes", "autre", "autres",
    "trop", "peu", "beaucoup", "déjà", "alors", "ainsi",
    "ici", "là", "jamais", "toujours", "souvent", "parfois",
    "maintenant", "hier", "demain", "fois",
    // Résidus d'élision (filtrés aussi par longueur ≤ 1)
    "l", "d", "n", "s", "c", "qu", "j", "m", "t",
    // Démonstratifs / divers
    "ça", "cela", "ceci", "oui", "non", "via",
];

/// Layer 2 — IT/ITSM stop words (~50 words)
const ITSM_STOP_WORDS: &[&str] = &[
    // Formules de politesse
    "bonjour", "bonsoir", "cordialement", "cdlt", "salutations",
    "merci", "svp", "veuillez", "madame", "monsieur", "cher", "chère",
    // Fillers ticket
    "objet", "ref", "référence", "ci-joint", "ci-dessous", "ci-dessus",
    "pj", "sujet", "info", "information",
    // Accusés de réception
    "reçu", "enregistré", "noté", "transmis", "transféré", "pris", "compte",
    // Mots ITSM à faible signal
    "ticket", "incident", "demande", "numéro", "dossier", "urgent", "priorité",
    "date", "problème", "question", "aide", "besoin", "solution", "réponse",
    // IT générique
    "système", "service", "utilisateur", "poste", "logiciel", "résolu", "clos",
    "resolu", "assigné",
];

/// Layer 3 — GLPI template phrases (removed before tokenisation)
const GLPI_PHRASES: &[&str] = &[
    "suite contact avec",
    "l'incident ou la demande",
    "les actions suivantes ont été effectuées",
    "pièce jointe liée lors de la création",
    "assigné au groupe",
    "assigné au technicien",
    "changement de statut",
    "nouveau ticket créé",
    "ticket résolu",
    "ticket clos",
    "en attente de",
    "validation demandée à",
    "suivi ajouté par",
    "tâche ajoutée par",
    "solution ajoutée par",
    "mise à jour du ticket",
    "ce ticket a été créé automatiquement",
    "merci de ne pas répondre à ce message",
    "satisfaction demandée le",
    "planifié le",
];

/// Layer 4 — Signature phrases (pre-defined, removed before tokenisation)
const SIGNATURE_PHRASES: &[&str] = &[
    "intervenant équipe support",
    "intervenant équipe réseau",
    "service desk",
    "support technique",
    "direction des systèmes d'information",
    "envoyé depuis",
    "ce message et ses pièces jointes",
    "n'imprimez ce mail que si nécessaire",
];

// ── StopWordFilter ────────────────────────────────────────────────────────────

/// Multi-layer stop-word filter for GLPI ticket preprocessing.
///
/// - **Layers 1+2** (`single_words`): French standard + IT/ITSM single-word stop words.
/// - **Layers 3+4** (`phrase_patterns`): GLPI template phrases + signatures removed *before*
///   tokenisation, with pre-compiled case-insensitive regexes.
pub struct StopWordFilter {
    /// Phrase patterns for layers 3+4 (lowercase, kept for debugging / inspection)
    phrase_patterns: Vec<String>,
    /// Pre-compiled case-insensitive regex per phrase pattern
    phrase_regexes: Vec<Regex>,
    /// Single-word stop words (lowercase) for layers 1+2+4-dynamic
    single_words: HashSet<String>,
}

impl StopWordFilter {
    /// Create a new filter with all hardcoded stop words and phrase patterns.
    pub fn new() -> Self {
        let mut single_words = HashSet::new();
        for &w in FRENCH_STOP_WORDS {
            single_words.insert(w.to_string());
        }
        for &w in ITSM_STOP_WORDS {
            single_words.insert(w.to_string());
        }

        let phrase_patterns: Vec<String> = GLPI_PHRASES
            .iter()
            .chain(SIGNATURE_PHRASES.iter())
            .map(|&s| s.to_string())
            .collect();

        let phrase_regexes: Vec<Regex> = phrase_patterns
            .iter()
            .filter_map(|p| Regex::new(&format!("(?i){}", regex::escape(p))).ok())
            .collect();

        StopWordFilter { phrase_patterns, phrase_regexes, single_words }
    }

    /// Remove GLPI template phrases and signatures from `text` (case-insensitive).
    ///
    /// Each matched region is replaced with a single space to preserve word boundaries.
    pub fn remove_phrases(&self, text: &str) -> String {
        let mut result = text.to_string();
        for re in &self.phrase_regexes {
            result = re.replace_all(&result, " ").to_string();
        }
        result
    }

    /// Return `true` if the lowercase `token` is a stop word (any layer).
    pub fn is_stop_word(&self, token: &str) -> bool {
        self.single_words.contains(token)
    }

    /// Add technician names as dynamic stop words (layer 4 — dynamic).
    ///
    /// Each whitespace-separated part of every name is lowercased and inserted into
    /// the single-word stop-word set so that technician names are excluded from TF-IDF.
    pub fn add_technician_names(&mut self, names: &[String]) {
        for name in names {
            for part in name.split_whitespace() {
                self.single_words.insert(part.to_lowercase());
            }
        }
    }
}

impl Default for StopWordFilter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Public helpers ────────────────────────────────────────────────────────────

/// Strip HTML tags and common HTML entities from `text`.
///
/// Matched regions are replaced with a single space to preserve word boundaries.
pub fn strip_html(text: &str) -> String {
    HTML_REGEX.replace_all(text, " ").to_string()
}

// ── Preprocessing pipeline ────────────────────────────────────────────────────

/// Preprocess a single ticket text through the full NLP pipeline:
///
/// 1. Strip HTML tags and entities.
/// 2. Remove GLPI template phrases (layers 3+4) via `filter`.
/// 3. Tokenize with charabia (word tokens only).
/// 4. Lowercase; filter tokens outside \[2, 30\] characters (removes elision residues).
/// 5. Filter stop words (layers 1+2+dynamic).
/// 6. Stem with Snowball French algorithm.
///
/// Returns a `Vec<String>` of clean stems ready for TF-IDF or clustering.
pub fn preprocess_text(text: &str, filter: &StopWordFilter) -> Vec<String> {
    let after_html = strip_html(text);
    let after_phrases = filter.remove_phrases(&after_html);

    let stemmer = Stemmer::create(Algorithm::French);

    after_phrases
        .as_str()
        .tokenize()
        .filter(|t| t.is_word())
        .map(|t| t.lemma().to_lowercase())
        .filter(|t: &String| t.len() >= 2 && t.len() <= 30)
        .filter(|t: &String| !filter.is_stop_word(t.as_str()))
        .map(|t: String| stemmer.stem(t.as_str()).to_string())
        .filter(|t: &String| t.len() >= 2)
        .collect()
}

/// Apply `preprocess_text` to every document in a corpus.
pub fn preprocess_corpus(texts: &[String], filter: &StopWordFilter) -> Vec<Vec<String>> {
    texts.iter().map(|t| preprocess_text(t, filter)).collect()
}

/// Like `preprocess_text`, but returns `(stem, original_word)` pairs.
///
/// The stems are used for TF-IDF grouping while the original words
/// are kept for building a reverse stem→word mapping.
pub fn preprocess_text_with_originals(
    text: &str,
    filter: &StopWordFilter,
) -> Vec<(String, String)> {
    let after_html = strip_html(text);
    let after_phrases = filter.remove_phrases(&after_html);

    let stemmer = Stemmer::create(Algorithm::French);

    after_phrases
        .as_str()
        .tokenize()
        .filter(|t| t.is_word())
        .map(|t| t.lemma().to_lowercase())
        .filter(|t: &String| t.len() >= 2 && t.len() <= 30)
        .filter(|t: &String| !filter.is_stop_word(t.as_str()))
        .map(|original: String| {
            let stem = stemmer.stem(original.as_str()).to_string();
            (stem, original)
        })
        .filter(|(stem, _)| stem.len() >= 2)
        .collect()
}

/// Build a reverse mapping from stems to the most frequent original word.
///
/// For each stem, counts how often each original word appears and picks
/// the most common one. E.g. stem "remplac" → "remplacer" (if most frequent).
pub fn build_stem_mapping(pairs: &[(String, String)]) -> HashMap<String, String> {
    let mut counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for (stem, original) in pairs {
        *counts
            .entry(stem.clone())
            .or_default()
            .entry(original.clone())
            .or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(stem, originals)| {
            let best = originals
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(word, _)| word)
                .unwrap_or_else(|| stem.clone());
            (stem, best)
        })
        .collect()
}

/// Resolve a stem back to its most frequent original word using the mapping.
///
/// Returns the original word if found in the mapping, otherwise the stem itself.
pub fn resolve_stem(stem: &str, mapping: &HashMap<String, String>) -> String {
    mapping.get(stem).cloned().unwrap_or_else(|| stem.to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let input = "<p>Bonjour <br/>monde</p> &amp; &lt;valeur&gt;";
        let output = strip_html(input);
        assert!(!output.contains('<'), "HTML open tags not removed");
        assert!(!output.contains('>'), "HTML close tags not removed");
        assert!(!output.contains("&amp;"), "HTML entity &amp; not removed");
        assert!(output.contains("Bonjour"), "Content word 'Bonjour' preserved");
        assert!(output.contains("monde"), "Content word 'monde' preserved");
    }

    #[test]
    fn test_stop_word_french() {
        let filter = StopWordFilter::new();
        assert!(filter.is_stop_word("le"), "'le' should be a stop word");
        assert!(filter.is_stop_word("la"), "'la' should be a stop word");
        assert!(filter.is_stop_word("les"), "'les' should be a stop word");
        assert!(!filter.is_stop_word("imprimante"), "'imprimante' is not a stop word");
    }

    #[test]
    fn test_stop_word_itsm() {
        let filter = StopWordFilter::new();
        assert!(filter.is_stop_word("ticket"), "'ticket' should be a stop word");
        assert!(filter.is_stop_word("incident"), "'incident' should be a stop word");
        assert!(filter.is_stop_word("cordialement"), "'cordialement' should be a stop word");
        assert!(!filter.is_stop_word("réseau"), "'réseau' is not a stop word");
    }

    #[test]
    fn test_remove_phrases() {
        let filter = StopWordFilter::new();
        let text = "Assigné au groupe DSI. Connexion VPN impossible.";
        let result = filter.remove_phrases(text);
        assert!(
            !result.to_lowercase().contains("assigné au groupe"),
            "GLPI template phrase should be removed"
        );
        assert!(
            result.contains("DSI") || result.contains("Connexion"),
            "Non-template content should be preserved"
        );
    }

    #[test]
    fn test_preprocess_text() {
        let filter = StopWordFilter::new();
        let text = "Les imprimantes du bureau ne fonctionnent plus depuis ce matin.";
        let tokens = preprocess_text(text, &filter);

        assert!(!tokens.iter().any(|t| t == "les"), "'les' should be filtered");
        assert!(!tokens.iter().any(|t| t == "du"), "'du' should be filtered");
        assert!(!tokens.iter().any(|t| t == "ne"), "'ne' should be filtered");
        assert!(!tokens.is_empty(), "Pipeline should produce content tokens");
        assert!(
            tokens.iter().all(|t| t.len() >= 2),
            "No token shorter than 2 chars after pipeline"
        );
    }

    #[test]
    fn test_add_technician_names() {
        let mut filter = StopWordFilter::new();
        assert!(!filter.is_stop_word("dupont"), "Name not yet a stop word");
        filter.add_technician_names(&[
            "Jean Dupont".to_string(),
            "Marie Martin".to_string(),
        ]);
        assert!(filter.is_stop_word("jean"), "'jean' should be a stop word after adding");
        assert!(filter.is_stop_word("dupont"), "'dupont' should be a stop word after adding");
        assert!(filter.is_stop_word("marie"), "'marie' should be a stop word after adding");
        assert!(filter.is_stop_word("martin"), "'martin' should be a stop word after adding");
    }

    #[test]
    fn test_short_tokens_filtered() {
        let filter = StopWordFilter::new();
        let tokens = preprocess_text("l'imprimante est cassée aujourd'hui", &filter);
        assert!(
            tokens.iter().all(|t| t.len() >= 2),
            "All tokens must have length >= 2 (elision residues like 'l' must be absent)"
        );
    }

    #[test]
    fn test_preprocess_text_with_originals() {
        let filter = StopWordFilter::new();
        let text = "Les imprimantes du bureau ne fonctionnent plus depuis ce matin.";
        let pairs = preprocess_text_with_originals(text, &filter);

        assert!(!pairs.is_empty(), "Should produce stem-original pairs");
        for (stem, original) in &pairs {
            assert!(stem.len() >= 2, "Stem too short: {stem}");
            assert!(original.len() >= 2, "Original too short: {original}");
        }
    }

    #[test]
    fn test_build_stem_mapping_picks_most_frequent() {
        let pairs = vec![
            ("remplac".to_string(), "remplacer".to_string()),
            ("remplac".to_string(), "remplacer".to_string()),
            ("remplac".to_string(), "remplacer".to_string()),
            ("remplac".to_string(), "remplacement".to_string()),
            ("appliqu".to_string(), "appliquer".to_string()),
            ("appliqu".to_string(), "application".to_string()),
            ("appliqu".to_string(), "appliquer".to_string()),
        ];
        let mapping = build_stem_mapping(&pairs);
        assert_eq!(mapping.get("remplac").unwrap(), "remplacer");
        assert_eq!(mapping.get("appliqu").unwrap(), "appliquer");
    }

    #[test]
    fn test_resolve_stem() {
        let mut mapping = HashMap::new();
        mapping.insert("remplac".to_string(), "remplacer".to_string());
        assert_eq!(resolve_stem("remplac", &mapping), "remplacer");
        assert_eq!(resolve_stem("inconnu", &mapping), "inconnu");
    }

    #[test]
    fn test_stemming() {
        let stemmer = Stemmer::create(Algorithm::French);
        let stem = stemmer.stem("imprimantes").to_string();
        assert!(
            stem.len() < "imprimantes".len(),
            "Snowball FR should shorten 'imprimantes' (got: '{}')",
            stem
        );
        assert!(!stem.is_empty(), "Stem must not be empty");
    }
}
