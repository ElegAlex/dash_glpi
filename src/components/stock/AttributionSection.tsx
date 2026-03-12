import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProfilingResult,
  AssignmentRecommendation,
  TechnicianSuggestion,
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
    <table className="w-full text-sm">
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
            <td className="py-1.5 font-[Source_Sans_3] font-semibold text-slate-800">
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

export function AttributionSection() {
  const [loading, setLoading] = useState(false);
  const [profilingResult, setProfilingResult] =
    useState<ProfilingResult | null>(null);
  const [recommendations, setRecommendations] = useState<
    AssignmentRecommendation[]
  >([]);
  const [error, setError] = useState<string | null>(null);

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
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-4">
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
