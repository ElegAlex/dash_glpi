# Claude Code Agent Teams : le deep dive définitif

**La fonctionnalité Agent Teams de Claude Code transforme un assistant de codage IA unique en un essaim coordonné d'instances Claude parallèles — chacune avec sa propre fenêtre de contexte, communiquant via un système de messagerie basé sur des fichiers, et se coordonnant via un tableau de tâches partagé.** Sortie le 5 février 2026 en "research preview" expérimentale aux côtés de Claude Opus 4.6, Agent Teams représente la productisation par Anthropic de patterns multi-agents que la communauté avait inventés avec des workarounds depuis des mois. La fonctionnalité permet à une session lead de créer des teammates indépendants qui travaillent simultanément, s'envoient des messages directs, contestent les conclusions des autres, et convergent vers des solutions — fondamentalement différent des simples subagents qui ne peuvent que reporter au parent. Ce rapport synthétise les conclusions de la documentation officielle, 30+ sources communautaires, dépôts GitHub, threads Hacker News, articles de blog et configurations de power users pour fournir l'analyse technique la plus approfondie possible.

---

## Comment Agent Teams fonctionne réellement sous le capot

L'architecture repose sur quatre composants étroitement intégrés. Un **Team Lead** (votre session Claude Code principale) crée l'équipe, décompose le travail, lance des **Teammates** (instances Claude Code indépendantes, chacune avec sa propre fenêtre de contexte de 200K tokens), coordonne via une **Shared Task List** (fichiers JSON sur disque avec suivi des dépendances et déblocage automatique), et route la communication inter-agents via un **Système Mailbox/Inbox** (messagerie basée sur des fichiers JSON).

Toute la coordination est basée sur le système de fichiers — pas de broker en mémoire, pas d'appels réseau entre agents :

```
~/.claude/teams/{team-name}/
├── config.json              # Métadonnées de l'équipe et liste des membres
└── inboxes/
    ├── team-lead.json       # Inbox du leader
    ├── worker-1.json        # Inbox du worker 1
    └── worker-2.json        # Inbox du worker 2

~/.claude/tasks/{team-name}/
├── 1.json                   # Tâche #1 (statut, propriétaire, dépendances)
├── 2.json                   # Tâche #2
└── 3.json                   # Tâche #3
```

Ce design basé fichiers a un avantage d'ingénierie crucial : **la récupération après crash est possible via des heartbeat timeouts** (timeout de 5 minutes qui libère les tâches abandonnées), et l'état de coordination est entièrement inspectable avec des outils Unix standard. Contraste net avec l'état graphique en mémoire (LangGraph) ou les handoffs conversationnels (CrewAI/AutoGen).

Activation de la fonctionnalité avec une seule variable d'environnement :

```json
// ~/.claude/settings.json
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  }
}
```

Les variables d'environnement injectées par teammate incluent `CLAUDE_CODE_TEAM_NAME`, `CLAUDE_CODE_AGENT_ID`, `CLAUDE_CODE_AGENT_NAME` et `CLAUDE_CODE_AGENT_TYPE`. Chaque teammate charge le même contexte projet (CLAUDE.md, serveurs MCP, skills) mais **n'hérite PAS de l'historique de conversation du lead** — c'est un choix de design critique qui garde le contexte de chaque teammate propre et focalisé.

### La distinction critique avec les subagents

Ce point est souvent mal compris. Subagents et Agent Teams servent des objectifs fondamentalement différents :

|Dimension|Subagents|Agent Teams|
|---|---|---|
|**Communication**|Report au parent uniquement|Messages directs entre eux via SendMessage|
|**Contexte**|Propre fenêtre, résultats retournés à l'appelant|Sessions entièrement indépendantes|
|**Coordination**|Le parent gère tout|Liste de tâches partagée avec auto-coordination|
|**Coût tokens**|Plus faible (~1.5x)|Plus élevé (~3-4x pour 3 teammates)|
|**Imbrication**|Ne peut pas créer de sub-subagents|Structure plate (pas d'équipes imbriquées)|
|**Idéal pour**|Tâches focalisées, fire-and-forget|Travail complexe nécessitant une collaboration active|

La règle de décision issue du consensus communautaire est simple : **vos workers ont-ils besoin de communiquer entre eux ?** Si oui, utilisez Agent Teams. Si non, utilisez des subagents.

---

## Les 13 opérations TeammateTool et les 7 primitives d'équipe

Le système TeammateTool, découvert initialement par le développeur Kieran Klaassen via une analyse `strings` du binaire Claude Code fin janvier 2026, expose **13 opérations distinctes** organisées en cinq catégories :

**Cycle de vie de l'équipe** : `spawnTeam` (crée config.json + répertoire de tâches), `discoverTeams` (lister les équipes disponibles), `cleanup` (supprimer les ressources de l'équipe du disque).

**Gestion des adhésions** : `requestJoin`, `approveJoin` (leader uniquement), `rejectJoin` (leader uniquement).

**Communication** : `write` (message direct à un teammate), `broadcast` (message à tous les teammates — coûteux car cela génère N messages pour N teammates).

**Approbation de plan** : `approvePlan` (leader uniquement), `rejectPlan` avec feedback (leader uniquement).

**Arrêt** : `requestShutdown` (leader uniquement), `approveShutdown` (teammate uniquement), `rejectShutdown` avec raison (teammate uniquement).

À un niveau supérieur, le workflow pratique utilise **sept primitives d'équipe** comme appels d'outils :

```javascript
// 1. Créer l'équipe
TeamCreate({ team_name: "blog-qa", description: "Équipe QA" })

// 2. Créer les tâches avec dépendances
TaskCreate({ subject: "Revoir module auth", description: "...", activeForm: "..." })
TaskCreate({ subject: "Revoir routes API", description: "...", blockedBy: ["1"] })

// 3. Lancer les teammates
Task({
  team_name: "blog-qa",
  name: "security-reviewer",
  subagent_type: "general-purpose",
  prompt: "Revoir le code d'authentification pour les vulnérabilités...",
  model: "sonnet",
  run_in_background: true
})

// 4. Auto-coordination via TaskList → TaskUpdate (claim) → travail → TaskUpdate (complete)
// 5. Messagerie inter-agents via SendMessage
// 6. Nettoyage via TeamDelete
```

Les états de tâches suivent le flux `pending` → `in_progress` → `completed`, avec les champs `blocks` et `blockedBy` permettant une exécution par vagues de dépendances. **Le verrouillage de fichier empêche les conditions de concurrence** lors de la revendication de tâches — quand deux teammates tentent simultanément de revendiquer la même tâche, un seul réussit.

**Types de subagents intégrés** pour la spécialisation de rôle au moment du spawn : **Bash** (commandes shell uniquement), **Explore** (outils en lecture seule, utilise Haiku par défaut pour l'efficacité de coût), **Plan** (lecture seule, pour la planification d'architecture), **general-purpose** (tous les outils disponibles), **claude-code-guide** (lecture seule + outils web) et **statusline-setup** (lecture + édition uniquement, modèle Sonnet).

---

## Modes d'affichage, raccourcis clavier et la réalité tmux

Agent Teams supporte trois backends d'affichage. **In-process** (par défaut dans les terminaux non-tmux) exécute tous les teammates dans un seul terminal avec `Shift+Haut/Bas` pour naviguer entre eux. **Split panes** donne à chaque teammate son propre pane tmux ou iTerm2 — la meilleure option pour 3+ teammates. **Auto** (par défaut) détecte l'environnement et utilise les split panes quand tmux est disponible.

```bash
# Forcer le backend tmux
export CLAUDE_CODE_SPAWN_BACKEND=tmux

# Ou configurer dans settings.json
{ "teammateMode": "tmux" }

# Ou par session
claude --teammate-mode in-process
```

Raccourcis clavier essentiels : `Shift+Haut/Bas` navigue entre les teammates, `Entrée` affiche la session du teammate sélectionné, `Échap` interrompt le tour d'un teammate, **`Ctrl+T` bascule la liste de tâches partagée**, et surtout, **`Shift+Tab` bascule le mode delegate** (restreignant le lead à la coordination uniquement).

Caveat majeur : **les split panes NE SONT PAS supportés dans le terminal intégré de VS Code, Windows Terminal ou Ghostty**. De multiples rapports communautaires (issue GitHub #23615) signalent des conditions de concurrence tmux lors du spawn de 4+ agents simultanément. Pour iTerm2, le setup nécessite l'installation du CLI `it2` (`uv tool install it2`) et l'activation de l'API Python dans les paramètres d'iTerm2.

---

## Les hooks de Quality Gate qui changent tout

Deux événements hook spécifiques aux Agent Teams, ajoutés dans la v2.1.33, permettent l'application automatisée de contrôles qualité :

**TeammateIdle** se déclenche quand un teammate est sur le point de passer en idle. Le code de sortie 2 envoie un feedback et maintient le teammate actif :

```json
{
  "hooks": {
    "TeammateIdle": [{
      "hooks": [{
        "type": "command",
        "command": "bash .claude/hooks/check-remaining-tasks.sh"
      }]
    }]
  }
}
```

**TaskCompleted** se déclenche quand une tâche est marquée complète. Le code de sortie 2 empêche la complétion et force l'agent à traiter le problème :

```json
{
  "hooks": {
    "TaskCompleted": [{
      "hooks": [{
        "type": "command",
        "command": "bash -c 'npm test || (echo \"Tests en échec\" && exit 2)'"
      }]
    }]
  }
}
```

Trois types de handlers sont disponibles : `command` (bash), `prompt` (évaluation Haiku) et `agent` (subagent avec outils). Le handler `prompt` est particulièrement puissant — il utilise un appel Haiku bon marché pour évaluer si les critères de qualité sont remplis, le rendant économique à exécuter sur chaque complétion de tâche.

---

## Cinq patterns d'orchestration validés par la communauté

### 1. Leader-Worker (le plus courant)

Un orchestrateur lance des spécialistes, assigne les tâches, collecte les résultats. Le pattern par défaut quand vous demandez à Claude de "créer une équipe". Fonctionne le mieux quand chaque worker gère un domaine indépendant (frontend, backend, tests).

### 2. Swarm (workers auto-assignés)

Les workers sont interchangeables et s'auto-assignent depuis une file de tâches partagée. La boucle de préambule worker du gist de Klaassen est l'implémentation canonique :

```
Tu es un worker swarm. BOUCLE :
1. Appelle TaskList() pour voir les tâches disponibles
2. Trouve une tâche pending, sans propriétaire, non bloquée
3. Revendique-la : TaskUpdate({ taskId: "X", owner: "TON_NOM" })
4. Fais le travail
5. Marque comme complétée : TaskUpdate({ taskId: "X", status: "completed" })
6. Envoie les résultats au team-lead
7. Répète jusqu'à ce qu'il ne reste plus de tâches
```

### 3. Council/Débat (adversarial)

Plusieurs agents proposent des solutions et contestent activement celles des autres. **Les LLM sont "significativement meilleurs en phase de revue qu'en phase d'implémentation"** (commentaire HN de frde_me) — ce pattern exploite cette asymétrie. Un utilisateur Hacker News (aqme28) a confirmé : rendre un modèle "adversarial" au sein de Claude améliore considérablement la qualité. Un agent fait la modification, l'autre pointe les failles, et ainsi de suite.

### 4. Pipeline (handoffs séquentiels)

Agent A → B → C, utilisant les dépendances `blockedBy`. Idéal pour les pipelines de production de contenu (chercheur → rédacteur → relecteur) ou les builds multi-phases (architecte → implémenteur → testeur).

### 5. Watchdog (moniteur + rollback)

Un agent worker exécute tandis qu'un agent observateur surveille les régressions, problèmes de sécurité ou échecs de tests, avec le pouvoir de déclencher un rollback.

---

## Les meta-prompts et configurations CLAUDE.md des power users

### Le meta-prompt "team plan"

C'est la technique la plus puissante découverte par la communauté. Ajoutez ceci à votre CLAUDE.md pour auto-générer des plans d'équipe à partir de demandes de fonctionnalités :

```markdown
## Génération de Team Plan

Quand je dis "team plan: [fonctionnalité]", génère une structure de tâches :

Pour chaque composant :
1. TaskCreate une tâche builder avec les fichiers spécifiques et critères d'acceptation
2. TaskCreate une tâche validator scopée à la vérification en lecture seule
3. TaskUpdate pour chaîner le validator derrière son builder

Après toutes les paires de composants, ajoute un validator d'intégration bloqué par TOUS les builders.

Formate chaque description de tâche avec :
- **Fichiers** : chemins exacts à créer ou lire
- **Critères** : conditions d'acceptation mesurables
- **Contraintes** : ce que cet agent ne doit PAS faire
```

Ensuite, dites simplement : _"team plan: ajouter un gestionnaire de webhooks Stripe."_ Claude génère le graphe complet de dépendances de tâches avec des paires builder-validator par composant et un validator d'intégration à la fin.

### Le pattern builder-validator

C'est le pattern le plus éprouvé pour les tâches d'implémentation :

**Prompt Builder** (scopé à la création) :

```
Tu es un agent builder. Ton travail :
1. Lis attentivement la description de la tâche
2. Implémente la solution dans les fichiers spécifiés
3. Exécute les tests pertinents
4. Marque ta tâche comme complétée

Règles :
- Ne modifie que les fichiers listés dans ta tâche
- Ne modifie pas les fichiers de test
- Si tu rencontres un blocage, documente-le et marque comme complété
```

**Prompt Validator** (scopé à la vérification) :

```
Tu es un agent validator. Ton travail :
1. Lis tous les fichiers que le builder a créés ou modifiés
2. Vérifie par rapport aux critères d'acceptation de la description de tâche
3. Exécute la suite de tests
4. Rapporte les problèmes en créant une nouvelle tâche si nécessaire

Règles :
- NE modifie AUCUN fichier source
- NE crée PAS de nouveau code d'implémentation
- Tu peux uniquement créer/mettre à jour des entrées de tâches pour signaler des problèmes
- Utilise uniquement Read et Bash (pour les tests) - jamais Edit ou Write
```

Quand un validator trouve des problèmes, il crée une tâche de correction → un nouveau builder la prend en charge → un nouveau validator se chaîne derrière. **Chaque cycle réduit le périmètre jusqu'à convergence.**

### CLAUDE.md pour le contexte d'équipe

Un CLAUDE.md bien structuré avec les frontières de modules réduit drastiquement le coût d'exploration par teammate :

```markdown
## Configuration Agent Team
Quand vous travaillez sur ce projet avec plusieurs agents :
- **Agent Backend** : Se concentre sur /src/server/. Suit les patterns middleware Express. Utilise TypeORM.
- **Agent Frontend** : Se concentre sur /src/client/. Utilise la bibliothèque de composants dans /src/client/components/shared/.
- **Agent Test** : Écrit les tests dans /tests/. Utilise Jest avec les helpers custom dans /tests/helpers/.
- **Agent Revue** : Revue de sécurité, typage, et conformité ESLint.

## Modules Indépendants
| Module | Répertoire | Notes |
| API | api/ | Chaque fichier est indépendant |
| CLI | src/ | Logique core |
| Website | docs/js/ | Contenu statique |

**Fichiers partagés (coordonner avant édition) :** package.json, tsconfig.json
```

---

## Ce qu'Anthropic a appris de 16 agents construisant un compilateur C

L'étude de cas définitive pour Agent Teams à grande échelle vient du chercheur Anthropic Nicholas Carlini, qui a assigné à **16 agents Claude parallèles** la construction d'un compilateur C en Rust from scratch. Les résultats sont stupéfiants :

- **~2 000 sessions Claude Code** sur deux semaines
- **2 milliards de tokens en entrée, 140 millions en sortie** — coût total sous **20 000 $**
- **100 000 lignes de code** produites
- Compile un **kernel Linux 6.9** bootable sur x86, ARM et RISC-V
- Compile aussi QEMU, FFmpeg, SQLite, PostgreSQL, Redis et Doom
- **99% de taux de réussite** sur la suite de tests torture GCC

Les leçons d'ingénierie de ce projet constituent les recommandations les plus autoritaires sur le codage multi-agents à grande échelle :

**Écrivez des tests de très haute qualité** — Claude optimise pour ce que le vérificateur contrôle. Si le harnais de test est bon, le code sera bon. **Concevez les tests pour Claude, pas pour les humains** — minimisez la pollution de la fenêtre de contexte en affichant des résumés, pas des milliers de lignes de sortie. **Gérez la cécité temporelle** — Claude ne peut pas mesurer le temps ; incluez des flags `--fast` qui testent un sous-ensemble aléatoire de 1-10% pour une itération rapide. **Facilitez le parallélisme** — utilisez un système de verrouillage basé fichiers où chaque agent revendique une tâche en écrivant un fichier de lock dans `current_tasks/` ; la synchronisation git résout les conflits. **Aucun agent d'orchestration n'était nécessaire** — chaque agent décidait indépendamment quoi travailler ensuite, choisissant le "prochain problème le plus évident".

---

## 13 limitations connues, anti-patterns et leçons durement acquises

La communauté a identifié une liste significative de limitations et pièges via l'usage en conditions réelles :

1. **Pas de reprise de session** : `/resume` et `/rewind` ne restaurent pas les teammates in-process. Après reprise, le lead peut essayer de messageer des teammates qui n'existent plus.
    
2. **Une seule équipe par session, pas d'équipes imbriquées** : Les teammates ne peuvent pas créer leurs propres équipes. La structure est plate.
    
3. **Les permissions se propagent** : Tous les teammates héritent des paramètres de permission du lead. On peut changer les modes individuels après le spawn, mais **on ne peut pas définir de modes par teammate au moment du spawn** (issue GitHub #24316).
    
4. **Le lead implémente au lieu de déléguer** : Sans le mode delegate (`Shift+Tab`), le lead prend fréquemment les tâches destinées aux teammates. C'est le mode de défaillance le plus couramment rapporté par la communauté.
    
5. **Les agents ignorent parfois la fonctionnalité** : Plusieurs utilisateurs HN (pjm331, oc1) ont rapporté que Claude ignorait les subagents disponibles et faisait le travail lui-même — "le problème principal est que les agents ne sont tout simplement pas utilisés."
    
6. **Crash par overflow de contexte** : Les agents chargeant trop de données atteignent les limites de contexte et deviennent incapables de `/compact`. Un rapport LessWrong décrit un agent superviseur crashant après avoir essayé d'aider un worker crashé, créant une défaillance en cascade.
    
7. **Retard dans le statut des tâches** : Les teammates échouent parfois à marquer les tâches comme complétées, les laissant bloquées en `in_progress` pour toujours.
    
8. **Overhead de coordination avec les grandes équipes** : Un auteur Medium (itsHabib) a constaté que tout mettre dans une seule grande équipe — "frontend, backend, infra, SRE" — ne fonctionnait pas : trop d'overhead de coordination, saignement de contexte entre les rôles, et agents faisant des hypothèses contradictoires.
    
9. **La consommation de tokens est réelle** : Un article DEV Community documente l'atteinte de la limite d'utilisation du plan Pro en **15 minutes** avec des agents parallèles, et les 5x tokens du plan Max à 100$ ont duré seulement **~75 minutes**.
    
10. **Bugs de routage de messages** : SendMessage peut réussir silencieusement quand le nom du destinataire ne correspond pas au nom de polling de l'inbox (issue #25135).
    
11. **L'arrêt peut être lent** : Les teammates finissent leur requête en cours avant de s'arrêter.
    
12. **Split panes non supportés dans les terminaux populaires** : Terminal intégré VS Code, Windows Terminal et Ghostty manquent tous de support.
    
13. **Problèmes d'ID de modèle Bedrock/Vertex** : Corrigé dans la v2.1.45, mais les versions antérieures avaient des décalages d'identifiants de modèles lors du spawn de teammates sur des fournisseurs API non-Anthropic.
    

### Anti-patterns critiques

La **décomposition centrée sur le problème** est l'anti-pattern le plus courant contre lequel Anthropic met en garde. Diviser par type de travail (un agent écrit les features, un autre les tests, un troisième fait la revue) crée un overhead de coordination constant — chaque handoff perd du contexte. La **décomposition centrée sur le contexte** divise par frontières de contexte : l'agent qui gère une feature devrait aussi gérer ses tests, parce qu'il possède déjà le contexte nécessaire.

Anthropic est direct à ce sujet : _"Nous avons vu des équipes investir des mois dans des architectures multi-agents élaborées pour découvrir qu'un meilleur prompting sur un seul agent atteignait des résultats équivalents."_

---

## Comparaison avec les frameworks concurrents

|Caractéristique|Agent Teams|CrewAI|AutoGen (AG2)|LangGraph|OpenAI Codex|
|---|---|---|---|---|---|
|**Architecture**|Lead-teammate natif avec liste de tâches partagée|Crews basés sur les rôles|Dialogue multi-agent conversationnel|Workflows à états basés sur des graphes|Agent unique autonome|
|**Setup**|1 var d'env, langage naturel|Config Python, définitions de rôles|Design complexe de flux conversationnel|Définition de graphe en code|N/A|
|**Comm inter-agents**|Peer-to-peer direct (SendMessage)|Handoffs structurés|Basé sur le dialogue|Passage d'état via les arêtes|N/A|
|**Support LLM**|Claude uniquement|Multi-fournisseur|Multi-fournisseur|Multi-fournisseur|OpenAI uniquement|
|**Intégration code**|Native (terminal, filesystem, git)|Usage général|Usage général|Usage général|Basé sandbox|
|**Maturité production**|Expérimental|Production|Production|Production|GA|

### Avantages uniques de l'approche native Claude Code

**Zero-configuration** : Une variable d'environnement et un prompt en langage naturel. Pas de framework à installer, pas de logique d'orchestration à concevoir. "Demandez juste une équipe" et Claude gère la décomposition, le spawn et la coordination.

**Communication peer-to-peer véritable** : Contrairement à la plupart des frameworks qui utilisent des handoffs structurés (CrewAI) ou du passage d'état (LangGraph), Agent Teams permettent la messagerie directe entre pairs. Les teammates peuvent contester les conclusions des autres en pleine tâche — pas juste du parallélisme, mais de la **coordination active**.

**Intégration profonde avec le codebase** : Chaque teammate a un accès complet au filesystem, terminal, git, serveurs MCP, CLAUDE.md et skills. La coordination native au filesystem signifie que tout est inspectable avec `cat` et `ls`.

**Spectre de complexité progressive** : Sessions solo → Subagents → Agent Teams → Claude Agent SDK pour des agents entièrement personnalisés. Choisissez le bon niveau pour le problème.

### Un bake-off réel : Agent Teams vs Claude-Flow

Derek Ashmore a mené une comparaison de type RFP et a constaté que Agent Teams produisait des **résultats de recherche plus minces** que Claude-Flow mais avec un **setup dramatiquement plus facile**. Le compromis est simplicité versus profondeur et personnalisation — Agent Teams gagne sur la facilité d'utilisation mais offre moins de contrôle granulaire sur le comportement des agents.

---

## L'écosystème d'outils et extensions communautaires

L'explosion d'outillage communautaire autour d'Agent Teams révèle à la fois la puissance de la fonctionnalité et ses lacunes :

|Outil|Source|Description|
|---|---|---|
|**Compound Engineering Plugin**|github.com/EveryInc/compound-engineering-plugin|Cycle plan → travail → revue → compound. 24 agents, 23 commandes de workflow. `/workflows:review` lance 12 subagents parallèles.|
|**Oh-My-ClaudeCode**|github.com/Yeachan-Heo/oh-my-claudecode|Orchestration teams-first avec `/omc-teams`. Supporte les fournisseurs mixtes (Claude + Codex + Gemini dans une équipe). Auto-resume sur rate limits.|
|**Ruflo**|github.com/ruvnet/ruflo|Plateforme d'orchestration entreprise avec 175+ outils MCP, intelligence de swarm, coordination queen, packaging Nix.|
|**Claude Colony**|github.com/MakingJamie/claude-colony|Multi-agent basé tmux avec @mentions. Manager à gauche, workers empilés à droite.|
|**Agent-Swarm**|github.com/desplega-ai/agent-swarm|Serveur MCP lead/workers conteneurisé Docker. "Les workers obtiennent un Ubuntu complet avec sudo."|
|**ZeroShot**|github.com/covibes/zeroshot|Clusters d'agents autonomes avec routage conductor et "agents de revue indépendants avec mandats séparés et droit de veto."|
|**VoltAgent Subagents**|github.com/VoltAgent/awesome-claude-code-subagents|Bibliothèque de 100+ définitions de subagents spécialisés.|
|**wshobson/agents**|github.com/wshobson/agents|112 agents, 16 orchestrateurs de workflow, 146 skills, 72 plugins.|
|**Klaassen Swarm Skill**|gist.github.com/kieranklaassen/4f2aba89594a4aea4ad64d753984b2ea|La référence SKILL.md canonique avec les 13 opérations, les 5 patterns, et du code copier-coller.|
|**claude-sneakpeek**|github.com/mikekelly/claude-sneakpeek|Accès pré-release au TeammateTool natif avant le lancement officiel.|
|**claude-config**|github.com/solatis/claude-config|Workflow de planification détaillée de 30-60 minutes pour résoudre les ambiguïtés avant l'exécution des agents.|

L'implémentation la plus extrême rapportée : **24 agents Claude Code simultanés** (utilisateur HN Shmungus) tournant sur du matériel local via un pipeline d'orchestration Rust natif Tokio routant à travers Mistral 7B local. Les agents communiquaient via des documents de gouvernance (CLAUDE.md, AGENTS.md) définissant la propriété des modules — **683 tests, 0 échecs, ratio test/production de 1.53:1**.

---

## Stratégies actionables d'optimisation des coûts

L'économie des tokens est la contrainte principale. Une **équipe de 3 teammates consomme environ 3-4x les tokens** d'une session unique faisant le même travail séquentiellement. La communauté a convergé sur ces stratégies d'optimisation :

**Stratégie de modèles mixtes** : Exécutez le lead sur **Opus** (raisonnement fort pour la coordination) et les teammates sur **Sonnet** (exécution plus rapide, moins chère). C'est le pattern le plus universellement recommandé.

**Planifier d'abord, paralléliser ensuite** : Le mode plan coûte **~10K tokens**. Une équipe mal dirigée coûte **500K+ tokens**. Utilisez toujours le mode plan pour créer un spec détaillé avant de spawner des teammates.

**Divulgation progressive pour CLAUDE.md** : Les skills ne chargent que les noms et descriptions au démarrage (~100 tokens chacun) ; les instructions complètes se chargent à la demande. La recherche de ClaudeFast a montré que cela récupère ~15 000 tokens par session (amélioration de 82% par rapport au chargement de tout dans CLAUDE.md).

**Approche par phases plutôt que grandes équipes** : L'auteur Medium itsHabib a constaté que des "petites équipes focalisées une phase à la fois" surperformaient drastiquement une seule grande équipe. Architecture (2 agents) → Backend (3 agents) → Frontend (1 agent) → Testing (2 agents).

**Trois teammates focalisés surperforment souvent cinq dispersés**. Le consensus communautaire place le sweet spot à **3-5 teammates** maximum, avec **5-6 tâches par teammate** pour garder tout le monde productif.

---

## L'histoire de la découverte et la timeline des versions

La timeline de découverte révèle une dynamique fascinante entre les hackers communautaires et le calendrier de release d'Anthropic :

- **18 décembre 2025** : Cyrus (@NicerInPerson) découvre pour la première fois TeammateTool dans le binaire Claude Code via une analyse `strings`
- **Janvier 2026** : Kieran Klaassen publie son gist complet Swarm Orchestration Skill avec les 13 opérations documentées ; Mike Kelly construit `claude-sneakpeek` pour contourner les feature flags
- **5 février 2026** : Anthropic lance officiellement Agent Teams aux côtés d'Opus 4.6 comme "research preview", documenté à code.claude.com/docs/en/agent-teams
- **v2.1.33** : Ajout des hooks TeammateIdle et TaskCompleted, restrictions de type d'agent au spawn, mémoire persistante pour les agents
- **v2.1.34** : Correction du crash quand les paramètres d'agent teams changeaient entre les rendus
- **v2.1.41/45** : Correction des échecs de teammates sur Bedrock/Vertex/Foundry par propagation des variables d'environnement du fournisseur API

Comme l'a noté paddo.dev, ce n'est pas la première fois qu'Anthropic productise des workarounds communautaires — avant TeammateTool, les développeurs avaient construit claude-flow, ccswarm et oh-my-claudecode. Anthropic a absorbé les patterns et les a livrés nativement. Comme l'a formulé un commentateur HN (joshribakoff) : _"Claude est déjà le meilleur orchestrateur pour Claude."_

---

## Conclusion : quand utiliser Agent Teams et quand s'abstenir

L'insight le plus profond de cette recherche est la recommandation contre-intuitive d'Anthropic elle-même : **la décomposition centrée sur le contexte bat la décomposition centrée sur le problème**. Ne divisez pas par rôle (un agent pour les features, un pour les tests, un pour la revue). Divisez par frontières de contexte — l'agent qui implémente une feature devrait aussi écrire ses tests, parce qu'il possède déjà le contexte nécessaire. Ne divisez que quand le contexte peut être véritablement isolé.

Agent Teams excelle pour les **tâches genuinement parallélisables et à haute valeur** : revue de code multi-perspectives, debugging avec hypothèses concurrentes, implémentation de features cross-layer avec des frontières de fichiers propres, et exploration de recherche où des angles multiples accélèrent la découverte. C'est activement contre-productif pour les tâches séquentielles, les éditions sur le même fichier, ou le travail avec de fortes interdépendances qui forcent une coordination constante.

La découverte la plus contre-intuitive des power users est l'**asymétrie entre implémentation et revue**. Les LLM sont significativement meilleurs en revue qu'en implémentation. Le pattern de débat adversarial — où les agents contestent le travail des autres — produit systématiquement des résultats de meilleure qualité que toute approche mono-agent. Cela signifie que l'usage le plus rentable d'Agent Teams n'est peut-être pas l'implémentation parallèle du tout, mais la **revue adversariale parallèle** du travail fait par d'autres agents ou des humains.

Commencez par une revue de code parallèle en mode delegate. Passez au debugging avec débat. N'essayez l'implémentation parallèle avec des frontières de fichiers strictes qu'ensuite. Les développeurs qui construisent aujourd'hui un muscle mémoire d'orchestration d'agents investissent dans une compétence qui se composera à mesure que ces outils mûriront — mais seulement s'ils résistent à la complexité séduisante de la sur-ingénierie de l'orchestration elle-même.

---

## Sources principales

- Documentation officielle : https://code.claude.com/docs/en/agent-teams
- Addy Osmani — Claude Code Swarms : https://addyosmani.com/blog/claude-code-agent-teams/
- Paddo.dev — Claude Code's Hidden Multi-Agent System : https://paddo.dev/blog/claude-code-hidden-swarm/
- Klaassen Swarm Orchestration Skill : https://gist.github.com/kieranklaassen/4f2aba89594a4aea4ad64d753984b2ea
- Klaassen Multi-Agent Orchestration System : https://gist.github.com/kieranklaassen/d2b35569be2c7f1412c64861a219d51f
- Alexop — From Tasks to Swarms : https://alexop.dev/posts/from-tasks-to-swarms-agent-teams-in-claude-code/
- ClaudeFast Guide complet : https://claudefa.st/blog/guide/agents/agent-teams
- ClaudeFast Builder-Validator Patterns : https://claudefa.st/blog/guide/agents/team-orchestration
- ClaudeFast Contrôles : https://claudefa.st/blog/guide/agents/agent-teams-controls
- ClaudeFast Best Practices : https://claudefa.st/blog/guide/agents/agent-teams-best-practices
- Anthropic — Building a C Compiler : https://www.anthropic.com/engineering/building-c-compiler
- Anthropic — When to use multi-agent systems : https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them
- Scott Spence — Unlock Swarm Mode : https://scottspence.com/posts/unlock-swarm-mode-in-claude-code
- Derek Ashmore — Agent Teams vs Claude-Flow Bake-Off : https://medium.com/@derekcashmore/claude-code-agent-teams-vs-claude-flow-a-real-world-bake-off-97e24f6ca9b9
- itsHabib — Trying Out Claude Code Teams : https://medium.com/@itsHabib/trying-out-claude-code-teams-e4c2a0eaf72f
- DEV Community — Sub Agents Token Burn : https://dev.to/onlineeric/claude-code-sub-agents-burn-out-your-tokens-4cd8
- Hacker News threads : #46902368, #46743908, #47099597, #46357942, #46525642, #46367037