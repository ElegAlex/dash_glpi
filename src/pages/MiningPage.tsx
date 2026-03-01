import { useState } from "react";
import { Search, GitBranch, AlertTriangle, Copy } from "lucide-react";
import { useInvoke } from "../hooks/useInvoke";
import WordCloud from "../components/mining/WordCloud";
import KeywordList from "../components/mining/KeywordList";
import ClusterView from "../components/mining/ClusterView";
import AnomalyList from "../components/mining/AnomalyList";
import DuplicateList from "../components/mining/DuplicateList";
import type {
  TextAnalysisResult,
  TextAnalysisRequest,
  ClusterResult,
  AnomalyAlert,
  DuplicatePair,
} from "../types/mining";

type Corpus = "titres" | "suivis";
type Scope = "global" | "group";
type MainTab = "keywords" | "clusters" | "anomalies" | "duplicates";

const CORPUS_LABELS: Record<Corpus, string> = {
  titres: "Titres",
  suivis: "Suivis",
};

const SCOPE_LABELS: Record<Scope, string> = {
  global: "Global",
  group: "Par groupe",
};

const MAIN_TABS: { key: MainTab; label: string; icon: React.ReactNode }[] = [
  { key: "keywords", label: "Mots-clés", icon: <Search size={15} /> },
  { key: "clusters", label: "Clusters", icon: <GitBranch size={15} /> },
  { key: "anomalies", label: "Anomalies", icon: <AlertTriangle size={15} /> },
  { key: "duplicates", label: "Doublons", icon: <Copy size={15} /> },
];

function StatCard({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex flex-col gap-1 rounded-lg border-l-4 border-[#0C419A] bg-[#f8f9fb] px-4 py-3">
      <span className="text-xs text-[#6e7891]">{label}</span>
      <span className="text-xl font-bold text-[#1a1f2e]">{value}</span>
    </div>
  );
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="rounded-xl border border-dashed border-[#cdd3df] bg-white py-16 text-center text-sm text-[#6e7891]">
      {message}
    </div>
  );
}

function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-[#ce0500] bg-[#fef2f2] px-4 py-3 text-sm text-[#af0400]">
      {message}
    </div>
  );
}

function Spinner({ label }: { label: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 gap-3 text-[#6e7891]">
      <div className="h-8 w-8 animate-spin rounded-full border-2 border-[#0C419A] border-t-transparent" />
      <span className="text-sm">{label}</span>
    </div>
  );
}

function MiningPage() {
  const [mainTab, setMainTab] = useState<MainTab>("keywords");
  const [corpus, setCorpus] = useState<Corpus>("titres");
  const [scope, setScope] = useState<Scope>("global");

  const analysisHook = useInvoke<TextAnalysisResult>();
  const clusterHook = useInvoke<ClusterResult>();
  const anomalyHook = useInvoke<AnomalyAlert[]>();
  const duplicateHook = useInvoke<DuplicatePair[]>();

  const handleAnalyze = () => {
    const request: TextAnalysisRequest = {
      corpus,
      scope,
      groupBy: scope === "group" ? "groupe_principal" : undefined,
      topN: 50,
    };
    analysisHook.execute("run_text_analysis", { request });
  };

  const handleCluster = () => {
    clusterHook.execute("get_clusters", { corpus: "titres", nClusters: 0 });
  };

  const handleAnomalies = () => {
    anomalyHook.execute("detect_anomalies", {});
  };

  const handleDuplicates = () => {
    duplicateHook.execute("detect_duplicates", {});
  };

  const result = analysisHook.data;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Search size={22} className="text-[#0C419A]" />
        <h1 className="text-2xl font-semibold text-[#1a1f2e]">Text Mining</h1>
      </div>

      {/* Main tabs */}
      <div className="border-b border-[#e2e6ee] bg-white">
        <div className="flex gap-0">
          {MAIN_TABS.map(({ key, label, icon }) => (
            <button
              key={key}
              onClick={() => setMainTab(key)}
              className={`flex items-center gap-2 px-5 py-3 text-sm font-medium transition-colors ${
                mainTab === key
                  ? "border-b-2 border-[#0C419A] text-[#0C419A]"
                  : "text-[#6e7891] hover:text-[#1a1f2e]"
              }`}
            >
              {icon}
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* ── Tab: Mots-clés ── */}
      {mainTab === "keywords" && (
        <div className="space-y-6">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex flex-wrap items-center gap-6">
              {/* Corpus sub-tabs */}
              <div className="flex gap-1 border-b border-[#e2e6ee]">
                {(Object.keys(CORPUS_LABELS) as Corpus[]).map((c) => (
                  <button
                    key={c}
                    onClick={() => setCorpus(c)}
                    className={`px-4 py-2 text-sm transition-colors ${
                      corpus === c
                        ? "border-b-2 border-[#0C419A] font-medium text-[#0C419A]"
                        : "text-[#6e7891] hover:text-[#1a1f2e]"
                    }`}
                  >
                    {CORPUS_LABELS[c]}
                  </button>
                ))}
              </div>

              {/* Scope sub-tabs */}
              <div className="flex gap-1 border-b border-[#e2e6ee]">
                {(Object.keys(SCOPE_LABELS) as Scope[]).map((s) => (
                  <button
                    key={s}
                    onClick={() => setScope(s)}
                    className={`px-4 py-2 text-sm transition-colors ${
                      scope === s
                        ? "border-b-2 border-[#0C419A] font-medium text-[#0C419A]"
                        : "text-[#6e7891] hover:text-[#1a1f2e]"
                    }`}
                  >
                    {SCOPE_LABELS[s]}
                  </button>
                ))}
              </div>
            </div>

            <button
              onClick={handleAnalyze}
              disabled={analysisHook.loading}
              className="rounded-lg bg-[#0C419A] px-4 py-2 text-sm font-medium text-white hover:bg-[#0a3480] disabled:opacity-50 transition-colors"
            >
              {analysisHook.loading ? "Analyse en cours…" : "Analyser le corpus"}
            </button>
          </div>

          {analysisHook.error && <ErrorBanner message={analysisHook.error} />}
          {analysisHook.loading && <Spinner label="Analyse en cours…" />}

          {!analysisHook.loading && !result && !analysisHook.error && (
            <EmptyState message="Cliquez sur Analyser le corpus pour lancer l'extraction de mots-clés" />
          )}

          {result && !analysisHook.loading && (
            <div className="space-y-6">
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
                <StatCard label="Documents" value={result.corpusStats.totalDocuments.toLocaleString("fr-FR")} />
                <StatCard label="Tokens" value={result.corpusStats.totalTokens.toLocaleString("fr-FR")} />
                <StatCard label="Vocabulaire" value={result.corpusStats.vocabularySize.toLocaleString("fr-FR")} />
                <StatCard label="Tokens / doc (moy.)" value={result.corpusStats.avgTokensPerDoc.toFixed(1)} />
              </div>

              {scope === "global" && result.keywords.length > 0 && (
                <div className="flex flex-col gap-4 lg:flex-row">
                  <div className="flex-1 rounded-xl border border-[#e2e6ee] bg-white p-4 shadow-sm overflow-hidden">
                    <h2 className="mb-3 text-sm font-semibold text-[#1a1f2e]">
                      Nuage de mots — Top 50
                    </h2>
                    <div className="flex justify-center">
                      <WordCloud keywords={result.keywords} width={700} height={380} />
                    </div>
                  </div>
                  <div className="w-full lg:w-80 shrink-0">
                    <KeywordList keywords={result.keywords} title="Top 20 mots-clés" maxItems={20} />
                  </div>
                </div>
              )}

              {scope === "group" && result.byGroup && result.byGroup.length > 0 && (
                <div className="space-y-4">
                  <h2 className="text-base font-semibold text-[#1a1f2e]">Mots-clés par groupe</h2>
                  <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
                    {result.byGroup.map((group) => (
                      <div key={group.groupName} className="rounded-xl border border-[#e2e6ee] bg-white p-4 shadow-sm">
                        <div className="mb-2 flex items-center justify-between">
                          <h3 className="text-sm font-semibold text-[#1a1f2e] truncate">{group.groupName}</h3>
                          <span className="shrink-0 ml-2 text-xs text-[#6e7891]">
                            {group.ticketCount} ticket{group.ticketCount > 1 ? "s" : ""}
                          </span>
                        </div>
                        <KeywordList keywords={group.keywords} title="" maxItems={10} />
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* ── Tab: Clusters ── */}
      {mainTab === "clusters" && (
        <div className="space-y-6">
          <div className="flex justify-end">
            <button
              onClick={handleCluster}
              disabled={clusterHook.loading}
              className="rounded-lg bg-[#0C419A] px-4 py-2 text-sm font-medium text-white hover:bg-[#0a3480] disabled:opacity-50 transition-colors"
            >
              {clusterHook.loading ? "Clustering en cours…" : "Lancer le clustering"}
            </button>
          </div>

          {clusterHook.error && <ErrorBanner message={clusterHook.error} />}
          {clusterHook.loading && <Spinner label="Clustering en cours…" />}

          {!clusterHook.loading && !clusterHook.data && !clusterHook.error && (
            <EmptyState message="Cliquez sur Lancer le clustering pour grouper les tickets par thématique" />
          )}

          {clusterHook.data && !clusterHook.loading && (
            <ClusterView
              clusters={clusterHook.data.clusters}
              silhouetteScore={clusterHook.data.silhouetteScore}
            />
          )}
        </div>
      )}

      {/* ── Tab: Anomalies ── */}
      {mainTab === "anomalies" && (
        <div className="space-y-6">
          <div className="flex justify-end">
            <button
              onClick={handleAnomalies}
              disabled={anomalyHook.loading}
              className="rounded-lg bg-[#0C419A] px-4 py-2 text-sm font-medium text-white hover:bg-[#0a3480] disabled:opacity-50 transition-colors"
            >
              {anomalyHook.loading ? "Détection en cours…" : "Détecter les anomalies"}
            </button>
          </div>

          {anomalyHook.error && <ErrorBanner message={anomalyHook.error} />}
          {anomalyHook.loading && <Spinner label="Détection d'anomalies en cours…" />}

          {!anomalyHook.loading && !anomalyHook.data && !anomalyHook.error && (
            <EmptyState message="Cliquez sur Détecter les anomalies pour identifier les tickets hors normes" />
          )}

          {anomalyHook.data && !anomalyHook.loading && (
            <AnomalyList anomalies={anomalyHook.data} />
          )}
        </div>
      )}

      {/* ── Tab: Doublons ── */}
      {mainTab === "duplicates" && (
        <div className="space-y-6">
          <div className="flex justify-end">
            <button
              onClick={handleDuplicates}
              disabled={duplicateHook.loading}
              className="rounded-lg bg-[#0C419A] px-4 py-2 text-sm font-medium text-white hover:bg-[#0a3480] disabled:opacity-50 transition-colors"
            >
              {duplicateHook.loading ? "Recherche en cours…" : "Chercher les doublons"}
            </button>
          </div>

          {duplicateHook.error && <ErrorBanner message={duplicateHook.error} />}
          {duplicateHook.loading && <Spinner label="Recherche de doublons en cours…" />}

          {!duplicateHook.loading && !duplicateHook.data && !duplicateHook.error && (
            <EmptyState message="Cliquez sur Chercher les doublons pour trouver les tickets similaires" />
          )}

          {duplicateHook.data && !duplicateHook.loading && (
            <DuplicateList duplicates={duplicateHook.data} />
          )}
        </div>
      )}
    </div>
  );
}

export default MiningPage;
