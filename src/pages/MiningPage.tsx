import { useState, useCallback, useEffect, useRef } from "react";
import { Search, GitBranch, AlertTriangle, Copy, FileText, Hash, ChevronDown } from "lucide-react";
import { useInvoke } from "../hooks/useInvoke";
import KeywordList from "../components/mining/KeywordList";
import CooccurrenceNetwork from "../components/mining/CooccurrenceNetwork";
import DrillDownPanel from "../components/mining/DrillDownPanel";
import ClusterView from "../components/mining/ClusterView";
import ClusterDetailPanel from "../components/mining/ClusterDetailPanel";
import AnomalyList from "../components/mining/AnomalyList";
import DuplicateList from "../components/mining/DuplicateList";
import { Card } from "../components/shared/Card";
import { KpiCard } from "../components/shared/KpiCard";
import type {
  TextAnalysisResult,
  TextAnalysisRequest,
  ClusterResult,
  ClusterInfo,
  ClusterDetail,
  AnomalyAlert,
  DuplicatePair,
  CooccurrenceResult,
  CooccurrenceRequest,
  TicketRef,
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
  { key: "keywords", label: "Mots-cles", icon: <Search size={15} /> },
  { key: "clusters", label: "Clusters", icon: <GitBranch size={15} /> },
  { key: "anomalies", label: "Anomalies", icon: <AlertTriangle size={15} /> },
  { key: "duplicates", label: "Doublons", icon: <Copy size={15} /> },
];

function EmptyState({ message }: { message: string }) {
  return (
    <div className="rounded-2xl bg-white py-16 text-center text-sm text-slate-400 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      {message}
    </div>
  );
}

function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
      {message}
    </div>
  );
}

function Spinner({ label }: { label: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 gap-3 text-slate-400">
      <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
      <span className="text-sm">{label}</span>
    </div>
  );
}

function MiningPage() {
  const [mainTab, setMainTab] = useState<MainTab>("keywords");
  const [corpus, setCorpus] = useState<Corpus>("titres");
  const [scope, setScope] = useState<Scope>("global");
  const [includeResolved, setIncludeResolved] = useState(false);
  const [clusterVivants, setClusterVivants] = useState(true);
  const [duplicateVivants, setDuplicateVivants] = useState(true);
  const [showNetwork, setShowNetwork] = useState(false);
  const [drillDown, setDrillDown] = useState<{ title: string; tickets: TicketRef[] } | null>(null);
  const [selectedCluster, setSelectedCluster] = useState<ClusterInfo | null>(null);

  const analysisHook = useInvoke<TextAnalysisResult>();
  const coocHook = useInvoke<CooccurrenceResult>();
  const clusterHook = useInvoke<ClusterResult>();
  const clusterDetailHook = useInvoke<ClusterDetail>();
  const anomalyHook = useInvoke<AnomalyAlert[]>();
  const duplicateHook = useInvoke<DuplicatePair[]>();

  const handleAnalyze = () => {
    const request: TextAnalysisRequest = {
      corpus,
      scope,
      groupBy: scope === "group" ? "groupe_principal" : undefined,
      topN: 100,
      includeResolved,
    };
    analysisHook.execute("run_text_analysis", { request });
  };

  const handleCluster = () => {
    clusterHook.execute("get_clusters", { corpus: "titres", nClusters: 0, vivantsOnly: clusterVivants });
  };

  const handleClusterClick = useCallback((cluster: ClusterInfo) => {
    setSelectedCluster(cluster);
    clusterDetailHook.execute("get_cluster_detail", { ticketIds: cluster.ticketIds });
  }, [clusterDetailHook]);

  const handleAnomalies = () => {
    anomalyHook.execute("detect_anomalies", {});
  };

  const handleDuplicates = () => {
    duplicateHook.execute("detect_duplicates", { vivantsOnly: duplicateVivants });
  };

  const handleCooccurrence = () => {
    const request: CooccurrenceRequest = {
      topNNodes: 80,
      maxEdges: 200,
      includeResolved,
    };
    coocHook.execute("get_cooccurrence_network", { request });
  };

  // Auto-refresh co-occurrence network when includeResolved changes
  const coocLoaded = useRef(false);
  useEffect(() => {
    if (coocHook.data) coocLoaded.current = true;
  }, [coocHook.data]);
  useEffect(() => {
    if (coocLoaded.current) {
      const request: CooccurrenceRequest = {
        topNNodes: 80,
        maxEdges: 200,
        includeResolved,
      };
      coocHook.execute("get_cooccurrence_network", { request });
    }
  }, [includeResolved]); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-refresh duplicates when toggle changes
  const dupLoaded = useRef(false);
  useEffect(() => {
    if (duplicateHook.data) dupLoaded.current = true;
  }, [duplicateHook.data]);
  useEffect(() => {
    if (dupLoaded.current) {
      duplicateHook.execute("detect_duplicates", { vivantsOnly: duplicateVivants });
    }
  }, [duplicateVivants]); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-refresh clusters when toggle changes
  const clusterLoaded = useRef(false);
  useEffect(() => {
    if (clusterHook.data) clusterLoaded.current = true;
  }, [clusterHook.data]);
  useEffect(() => {
    if (clusterLoaded.current) {
      clusterHook.execute("get_clusters", { corpus: "titres", nClusters: 0, vivantsOnly: clusterVivants });
    }
  }, [clusterVivants]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleNodeClick = useCallback(
    (word: string) => {
      if (!coocHook.data) return;
      const tickets = coocHook.data.ticketMap[word] ?? [];
      setDrillDown({ title: `Tickets contenant "${word}"`, tickets });
    },
    [coocHook.data],
  );

  const handleEdgeClick = useCallback(
    (source: string, target: string) => {
      if (!coocHook.data) return;
      const ticketsA = new Set((coocHook.data.ticketMap[source] ?? []).map((t) => t.id));
      const ticketsB = coocHook.data.ticketMap[target] ?? [];
      const intersection = ticketsB.filter((t) => ticketsA.has(t.id));
      setDrillDown({
        title: `Tickets contenant "${source}" et "${target}"`,
        tickets: intersection,
      });
    },
    [coocHook.data],
  );

  const result = analysisHook.data;

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Text Mining
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Analyse textuelle des tickets par TF-IDF, clustering et detection d'anomalies
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {/* Main tabs */}
        <div className="animate-fade-slide-up">
          <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] overflow-hidden">
            <div className="flex gap-0">
              {MAIN_TABS.map(({ key, label, icon }) => (
                <button
                  key={key}
                  onClick={() => setMainTab(key)}
                  className={`flex items-center gap-2 px-5 py-3 text-sm font-medium transition-colors ${
                    mainTab === key
                      ? "border-b-2 border-primary-500 text-primary-500"
                      : "text-slate-400 hover:text-slate-800"
                  }`}
                >
                  {icon}
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Tab: Mots-cles */}
        {mainTab === "keywords" && (
          <div className="space-y-6">
            <div className="flex flex-wrap items-center justify-between gap-4">
              <div className="flex flex-wrap items-center gap-6">
                {/* Corpus sub-tabs */}
                <div className="flex gap-1 bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-1">
                  {(Object.keys(CORPUS_LABELS) as Corpus[]).map((c) => (
                    <button
                      key={c}
                      onClick={() => setCorpus(c)}
                      className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                        corpus === c
                          ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                          : "text-slate-500 hover:text-slate-800"
                      }`}
                    >
                      {CORPUS_LABELS[c]}
                    </button>
                  ))}
                </div>

                {/* Scope sub-tabs */}
                <div className="flex gap-1 bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-1">
                  {(Object.keys(SCOPE_LABELS) as Scope[]).map((s) => (
                    <button
                      key={s}
                      onClick={() => setScope(s)}
                      className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                        scope === s
                          ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                          : "text-slate-500 hover:text-slate-800"
                      }`}
                    >
                      {SCOPE_LABELS[s]}
                    </button>
                  ))}
                </div>

                {/* Toggle vivants / tous */}
                <div className="flex gap-1 bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-1">
                  <button
                    onClick={() => setIncludeResolved(false)}
                    className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                      !includeResolved
                        ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                        : "text-slate-500 hover:text-slate-800"
                    }`}
                  >
                    En cours
                  </button>
                  <button
                    onClick={() => setIncludeResolved(true)}
                    className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                      includeResolved
                        ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                        : "text-slate-500 hover:text-slate-800"
                    }`}
                  >
                    Tous les tickets
                  </button>
                </div>
              </div>

              <button
                onClick={handleAnalyze}
                disabled={analysisHook.loading}
                className="rounded-xl bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50 transition-colors shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
              >
                {analysisHook.loading ? "Analyse en cours..." : "Analyser le corpus"}
              </button>
            </div>

            {analysisHook.error && <ErrorBanner message={analysisHook.error} />}
            {analysisHook.loading && <Spinner label="Analyse en cours..." />}

            {!analysisHook.loading && !result && !analysisHook.error && (
              <EmptyState message="Cliquez sur Analyser le corpus pour lancer l'extraction de mots-cles" />
            )}

            {result && !analysisHook.loading && (
              <div className="space-y-6">
                <div className="grid grid-cols-2 gap-5 sm:grid-cols-4">
                  <KpiCard
                    label="Documents"
                    value={result.corpusStats.totalDocuments}
                    icon={<FileText size={18} className="text-primary-500" />}
                    accentColor="#0C419A"
                  />
                  <KpiCard
                    label="Tokens"
                    value={result.corpusStats.totalTokens}
                    icon={<Hash size={18} className="text-emerald-600" />}
                    accentColor="#2E7D32"
                  />
                  <KpiCard
                    label="Vocabulaire"
                    value={result.corpusStats.vocabularySize}
                    icon={<Search size={18} className="text-purple-600" />}
                    accentColor="#6A1B9A"
                  />
                  <KpiCard
                    label="Tokens / doc"
                    value={result.corpusStats.avgTokensPerDoc.toFixed(1)}
                    accentColor="#FF8F00"
                  />
                </div>

                {scope === "global" && result.keywords.length > 0 && (
                  <div className="space-y-5">
                    <KeywordList
                      keywords={result.keywords}
                      title="Mots-cles extraits"
                      maxItems={30}
                      onKeywordClick={(word) => {
                        const tickets = result.ticketMap[word] ?? [];
                        setDrillDown({ title: `Tickets contenant "${word}"`, tickets });
                      }}
                    />

                    <Card className="overflow-hidden">
                      <div className="flex items-center justify-between">
                        <button
                          onClick={() => setShowNetwork(!showNetwork)}
                          className="flex items-center gap-2 text-left"
                        >
                          <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700">
                            Reseau de co-occurrences
                          </h2>
                          <ChevronDown
                            size={18}
                            className={`text-slate-400 transition-transform duration-200 ${
                              showNetwork ? "rotate-180" : ""
                            }`}
                          />
                        </button>
                        {showNetwork && (
                          <button
                            onClick={handleCooccurrence}
                            disabled={coocHook.loading}
                            className="rounded-xl bg-primary-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-primary-600 disabled:opacity-50 transition-colors"
                          >
                            {coocHook.loading ? "Generation..." : coocHook.data ? "Recalculer" : "Generer le reseau"}
                          </button>
                        )}
                      </div>
                      {showNetwork && (
                        <div className="mt-4">
                          {coocHook.loading && <Spinner label="Calcul des co-occurrences..." />}
                          {coocHook.error && <ErrorBanner message={coocHook.error} />}
                          {coocHook.data && !coocHook.loading && (
                            <CooccurrenceNetwork
                              data={coocHook.data}
                              height={500}
                              onNodeClick={handleNodeClick}
                              onEdgeClick={handleEdgeClick}
                            />
                          )}
                          {!coocHook.data && !coocHook.loading && !coocHook.error && (
                            <p className="py-8 text-center text-sm text-slate-400">
                              Cliquez sur Generer le reseau pour visualiser les connexions entre mots
                            </p>
                          )}
                        </div>
                      )}
                    </Card>
                  </div>
                )}

                {scope === "group" && result.byGroup && result.byGroup.length > 0 && (
                  <div className="space-y-4">
                    <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700">Mots-cles par groupe</h2>
                    <div className="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-3">
                      {result.byGroup.map((group) => (
                        <Card key={group.groupName}>
                          <div className="mb-2 flex items-center justify-between">
                            <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 truncate">{group.groupName}</h3>
                            <span className="shrink-0 ml-2 text-xs text-slate-400">
                              {group.ticketCount} ticket{group.ticketCount > 1 ? "s" : ""}
                            </span>
                          </div>
                          <KeywordList keywords={group.keywords} title="" maxItems={10} />
                        </Card>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* Tab: Clusters */}
        {mainTab === "clusters" && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div className="flex gap-1 bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-1">
                <button
                  onClick={() => setClusterVivants(true)}
                  className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                    clusterVivants
                      ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                      : "text-slate-500 hover:text-slate-800"
                  }`}
                >
                  En cours
                </button>
                <button
                  onClick={() => setClusterVivants(false)}
                  className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                    !clusterVivants
                      ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                      : "text-slate-500 hover:text-slate-800"
                  }`}
                >
                  Tous les tickets
                </button>
              </div>
              <button
                onClick={handleCluster}
                disabled={clusterHook.loading}
                className="rounded-xl bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50 transition-colors shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
              >
                {clusterHook.loading ? "Clustering en cours..." : "Lancer le clustering"}
              </button>
            </div>

            {clusterHook.error && <ErrorBanner message={clusterHook.error} />}
            {clusterHook.loading && <Spinner label="Clustering en cours..." />}

            {!clusterHook.loading && !clusterHook.data && !clusterHook.error && (
              <EmptyState message="Cliquez sur Lancer le clustering pour grouper les tickets par thematique" />
            )}

            {clusterHook.data && !clusterHook.loading && (
              <ClusterView
                clusters={clusterHook.data.clusters}
                silhouetteScore={clusterHook.data.silhouetteScore}
                onClusterClick={handleClusterClick}
              />
            )}
          </div>
        )}

        {/* Tab: Anomalies */}
        {mainTab === "anomalies" && (
          <div className="space-y-6">
            <div className="flex justify-end">
              <button
                onClick={handleAnomalies}
                disabled={anomalyHook.loading}
                className="rounded-xl bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50 transition-colors shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
              >
                {anomalyHook.loading ? "Detection en cours..." : "Detecter les anomalies"}
              </button>
            </div>

            {anomalyHook.error && <ErrorBanner message={anomalyHook.error} />}
            {anomalyHook.loading && <Spinner label="Detection d'anomalies en cours..." />}

            {!anomalyHook.loading && !anomalyHook.data && !anomalyHook.error && (
              <EmptyState message="Cliquez sur Detecter les anomalies pour identifier les tickets hors normes" />
            )}

            {anomalyHook.data && !anomalyHook.loading && (
              <AnomalyList anomalies={anomalyHook.data} />
            )}
          </div>
        )}

        {/* Tab: Doublons */}
        {mainTab === "duplicates" && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div className="flex gap-1 bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-1">
                <button
                  onClick={() => setDuplicateVivants(true)}
                  className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                    duplicateVivants
                      ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                      : "text-slate-500 hover:text-slate-800"
                  }`}
                >
                  En cours
                </button>
                <button
                  onClick={() => setDuplicateVivants(false)}
                  className={`px-4 py-2 text-sm rounded-xl transition-all duration-150 ${
                    !duplicateVivants
                      ? "bg-primary-500 text-white font-medium shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
                      : "text-slate-500 hover:text-slate-800"
                  }`}
                >
                  Tous les tickets
                </button>
              </div>
              <button
                onClick={handleDuplicates}
                disabled={duplicateHook.loading}
                className="rounded-xl bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50 transition-colors shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
              >
                {duplicateHook.loading ? "Recherche en cours..." : "Chercher les doublons"}
              </button>
            </div>

            {duplicateHook.error && <ErrorBanner message={duplicateHook.error} />}
            {duplicateHook.loading && <Spinner label="Recherche de doublons en cours..." />}

            {!duplicateHook.loading && !duplicateHook.data && !duplicateHook.error && (
              <EmptyState message="Cliquez sur Chercher les doublons pour trouver les tickets similaires" />
            )}

            {duplicateHook.data && !duplicateHook.loading && (
              <DuplicateList duplicates={duplicateHook.data} />
            )}
          </div>
        )}
      </div>

      {drillDown && (
        <DrillDownPanel
          title={drillDown.title}
          tickets={drillDown.tickets}
          onClose={() => setDrillDown(null)}
        />
      )}

      {selectedCluster && (
        <ClusterDetailPanel
          cluster={selectedCluster}
          detail={clusterDetailHook.data!}
          loading={clusterDetailHook.loading}
          error={clusterDetailHook.error}
          onClose={() => setSelectedCluster(null)}
        />
      )}
    </div>
  );
}

export default MiningPage;
