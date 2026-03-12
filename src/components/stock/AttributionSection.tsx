import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProfilingResult,
  AssignmentRecommendation,
  TechnicianSuggestion,
  UnassignedTicketStats,
} from "../../types/recommandation";

function ScoreBar({ score }: { score: number }) {
  const pct = Math.round(score * 100);
  const color =
    score > 0.6
      ? "bg-emerald-500"
      : score > 0.3
        ? "bg-amber-500"
        : "bg-red-500";

  return (
    <div className="flex items-center gap-2">
      <div className="w-16 h-1.5 bg-slate-100 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="font-[DM_Sans] font-semibold tabular-nums text-sm">
        {score.toFixed(2)}
      </span>
    </div>
  );
}

function SuggestionTable({
  suggestions,
}: {
  suggestions: TechnicianSuggestion[];
}) {
  if (suggestions.length === 0) {
    return (
      <p className="text-sm text-slate-400 italic font-[Source_Sans_3]">
        Aucune suggestion (scores trop faibles)
      </p>
    );
  }

  return (
    <table className="w-full text-sm table-fixed">
      <colgroup>
        <col className="w-[40px]" />
        <col />
        <col className="w-[80px]" />
        <col className="w-[70px]" />
        <col className="w-[140px]" />
      </colgroup>
      <thead>
        <tr className="text-left">
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            #
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Technicien
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Comp.
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Stock
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Score final
          </th>
        </tr>
      </thead>
      <tbody>
        {suggestions.map((s, idx) => (
          <tr
            key={s.technicien}
            className="hover:bg-[#0C419A]/[0.04] transition-colors"
          >
            <td className="py-1.5 font-[DM_Sans] font-semibold text-slate-400">
              {idx + 1}
            </td>
            <td className="py-1.5 font-[Source_Sans_3] font-semibold text-slate-800 truncate">
              {s.technicien}
            </td>
            <td className="py-1.5 font-[DM_Sans] font-semibold tabular-nums">
              {s.scoreCompetence.toFixed(2)}
            </td>
            <td className="py-1.5 font-[DM_Sans] font-semibold tabular-nums">
              {s.stockActuel}
            </td>
            <td className="py-1.5">
              <ScoreBar score={s.scoreFinal} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function KpiCard({
  label,
  value,
  subtitle,
  accentColor,
  icon,
}: {
  label: string;
  value: string;
  subtitle?: string;
  accentColor: string;
  icon: React.ReactNode;
}) {
  return (
    <div
      className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
                 hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]
                 hover:-translate-y-0.5 transition-all duration-200 p-6 relative overflow-hidden"
    >
      <div
        className="absolute top-0 left-0 right-0 h-[3px]"
        style={{ background: accentColor }}
      />
      <div className="flex items-start justify-between mb-3">
        <p className="text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
          {label}
        </p>
        <div className="text-slate-300">{icon}</div>
      </div>
      <p className="text-4xl font-bold font-[DM_Sans] tracking-tight text-slate-800">
        {value}
      </p>
      {subtitle && (
        <p className="text-sm text-slate-400 font-[Source_Sans_3] mt-1">
          {subtitle}
        </p>
      )}
    </div>
  );
}

export function AttributionSection() {
  const [loading, setLoading] = useState(false);
  const [profilingResult, setProfilingResult] =
    useState<ProfilingResult | null>(null);
  const [recommendations, setRecommendations] = useState<
    AssignmentRecommendation[]
  >([]);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<UnassignedTicketStats | null>(null);

  useEffect(() => {
    invoke<UnassignedTicketStats>("get_unassigned_ticket_stats_cmd")
      .then(setStats)
      .catch(() => {});
  }, []);

  // Computed KPIs from recommendations
  const couvertureIa =
    recommendations.length > 0
      ? Math.round(
          (recommendations.filter(
            (r) => r.suggestions.length > 0 && r.suggestions[0].scoreFinal > 0.3
          ).length /
            recommendations.length) *
            100
        )
      : null;

  const scoreMoyenTop1 =
    recommendations.length > 0
      ? recommendations.reduce(
          (sum, r) =>
            sum + (r.suggestions.length > 0 ? r.suggestions[0].scoreFinal : 0),
          0
        ) / recommendations.length
      : null;

  async function handleAnalyze() {
    setLoading(true);
    setError(null);
    try {
      const profiling = await invoke<ProfilingResult>(
        "build_technician_profiles"
      );
      setProfilingResult(profiling);

      if (profiling.profilesCount === 0) {
        setRecommendations([]);
        setLoading(false);
        return;
      }

      const recs = await invoke<AssignmentRecommendation[]>(
        "get_assignment_recommendations",
        { request: {} }
      );
      setRecommendations(recs);

      // Refresh stats after analysis
      const freshStats = await invoke<UnassignedTicketStats>(
        "get_unassigned_ticket_stats_cmd"
      );
      setStats(freshStats);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold font-[DM_Sans] text-slate-800">
            Attribution intelligente
          </h2>
          <p className="text-sm text-slate-400 font-[Source_Sans_3]">
            Suggestions d'attribution basées sur les compétences et la charge
          </p>
        </div>
        <button
          onClick={handleAnalyze}
          disabled={loading}
          className="px-5 py-2.5 bg-[#0C419A] text-white font-[DM_Sans] font-semibold
                     rounded-xl hover:bg-[#082A66] transition-colors disabled:opacity-50
                     shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
                     hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
        >
          {loading ? (
            <span className="flex items-center gap-2">
              <svg
                className="animate-spin h-4 w-4"
                viewBox="0 0 24 24"
                fill="none"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                  className="opacity-25"
                />
                <path
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  className="opacity-75"
                />
              </svg>
              Analyse en cours...
            </span>
          ) : (
            "Analyser"
          )}
        </button>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-4 gap-5">
        <KpiCard
          label="Tickets non assignés"
          value={stats ? String(stats.count) : "—"}
          subtitle="Vivants sans technicien"
          accentColor="#C62828"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M18 18.72a9.094 9.094 0 0 0 3.741-.479 3 3 0 0 0-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0 1 12 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 0 1 6 18.719m12 0a5.971 5.971 0 0 0-.941-3.197m0 0A5.995 5.995 0 0 0 12 12.75a5.995 5.995 0 0 0-5.058 2.772m0 0a3 3 0 0 0-4.681 2.72 8.986 8.986 0 0 0 3.74.477m.94-3.197a5.971 5.971 0 0 0-.94 3.197M15 6.75a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm6 3a2.25 2.25 0 1 1-4.5 0 2.25 2.25 0 0 1 4.5 0Zm-13.5 0a2.25 2.25 0 1 1-4.5 0 2.25 2.25 0 0 1 4.5 0Z" />
            </svg>
          }
        />
        <KpiCard
          label="Âge moyen"
          value={stats ? `${Math.round(stats.ageMoyenJours)}j` : "—"}
          subtitle="Des tickets non assignés"
          accentColor="#E65100"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
            </svg>
          }
        />
        <KpiCard
          label="Couverture IA"
          value={couvertureIa !== null ? `${couvertureIa}%` : "—"}
          subtitle="Tickets avec suggestion > 0.3"
          accentColor="#2E7D32"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 0 0-2.455 2.456ZM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 0 0-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 0 0 1.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 0 0 1.423 1.423l1.183.394-1.183.394a2.25 2.25 0 0 0-1.423 1.423Z" />
            </svg>
          }
        />
        <KpiCard
          label="Score moyen top 1"
          value={scoreMoyenTop1 !== null ? scoreMoyenTop1.toFixed(2) : "—"}
          subtitle="Confiance moyenne meilleure suggestion"
          accentColor="#0C419A"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z" />
            </svg>
          }
        />

      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-50 text-red-700 rounded-2xl p-4 font-[Source_Sans_3]">
          {error}
        </div>
      )}

      {/* Profiling status banner */}
      {profilingResult && (
        <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-4">
          <div className="flex items-center gap-3 text-sm font-[Source_Sans_3]">
            <span className="w-2 h-2 rounded-full bg-emerald-500" />
            <span className="text-slate-600">
              Profils calculés :{" "}
              <span className="font-semibold text-slate-800">
                {profilingResult.profilesCount} techniciens
              </span>
              ,{" "}
              <span className="font-semibold text-slate-800">
                {profilingResult.nbTicketsAnalysed} tickets
              </span>{" "}
              analysés
            </span>
            <span className="text-slate-400">
              Période : {profilingResult.periodeFrom} &rarr;{" "}
              {profilingResult.periodeTo}
            </span>
          </div>
        </div>
      )}

      {/* No profiles state */}
      {profilingResult && profilingResult.profilesCount === 0 && (
        <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-8 text-center">
          <div className="text-amber-500 text-3xl mb-2">&#9888;</div>
          <p className="text-slate-600 font-[Source_Sans_3]">
            Aucun ticket résolu trouvé dans les 6 derniers mois.
            <br />
            Impossible de construire des profils de compétence.
          </p>
        </div>
      )}

      {/* No unassigned tickets */}
      {profilingResult &&
        profilingResult.profilesCount > 0 &&
        recommendations.length === 0 &&
        !loading && (
          <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-8 text-center">
            <div className="text-emerald-500 text-3xl mb-2">&#10003;</div>
            <p className="text-slate-600 font-[Source_Sans_3]">
              Aucun ticket non attribué — tous les tickets vivants ont un
              technicien assigné.
            </p>
          </div>
        )}

      {/* Recommendation cards */}
      {recommendations.length > 0 && (
        <div className="space-y-4">
          {recommendations.map((rec) => (
            <div
              key={rec.ticketId}
              className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
                         hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]
                         transition-shadow duration-200 p-6"
            >
              <div className="mb-3">
                <div className="flex items-baseline gap-2">
                  <span className="text-xs font-[DM_Sans] font-semibold text-slate-400">
                    #{rec.ticketId}
                  </span>
                  <h3 className="font-[DM_Sans] font-semibold text-slate-800">
                    {rec.ticketTitre}
                  </h3>
                </div>
                {rec.ticketCategorie && (
                  <span className="text-xs text-slate-400 font-[Source_Sans_3]">
                    {rec.ticketCategorie}
                  </span>
                )}
              </div>
              <SuggestionTable suggestions={rec.suggestions} />
            </div>
          ))}
          <p className="text-xs text-slate-400 font-[Source_Sans_3] text-center">
            {recommendations.length} ticket
            {recommendations.length > 1 ? "s" : ""} non attribué
            {recommendations.length > 1 ? "s" : ""}
          </p>
        </div>
      )}
    </div>
  );
}
