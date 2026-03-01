import type { DuplicatePair } from "../../types/mining";

interface DuplicateListProps {
  duplicates: DuplicatePair[];
}

function SimilarityBar({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  const color = pct >= 90 ? "#C62828" : pct >= 75 ? "#FF8F00" : "#1565C0";
  return (
    <div className="flex items-center gap-2">
      <div className="h-2 flex-1 overflow-hidden rounded-full bg-slate-100">
        <div
          className="h-2 rounded-full transition-all"
          style={{ width: `${pct}%`, backgroundColor: color }}
        />
      </div>
      <span
        className="shrink-0 rounded-lg px-2 py-0.5 text-xs font-semibold"
        style={{ backgroundColor: `${color}12`, color }}
      >
        {pct}%
      </span>
    </div>
  );
}

export default function DuplicateList({ duplicates: duplicates }: DuplicateListProps) {
  const sorted = [...duplicates].sort((a, b) => b.similarity - a.similarity);

  return (
    <div className="space-y-4">
      <p className="text-sm text-slate-500">
        <strong className="text-slate-800 font-[DM_Sans]">{duplicates.length}</strong> doublon
        {duplicates.length !== 1 ? "s" : ""} potentiel{duplicates.length !== 1 ? "s" : ""} detecte
        {duplicates.length !== 1 ? "s" : ""}
      </p>

      {sorted.length === 0 && (
        <div className="rounded-2xl bg-white py-10 text-center text-sm text-slate-400 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
          Aucun doublon potentiel detecte
        </div>
      )}

      <div className="space-y-3">
        {sorted.map((pair) => (
          <div
            key={`${pair.ticketAId}-${pair.ticketBId}`}
            className="rounded-2xl bg-white p-4 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
          >
            <div className="mb-3 grid grid-cols-2 gap-3">
              <div className="rounded-xl bg-slate-50 px-3 py-2">
                <span className="block text-xs font-semibold text-primary-500 font-[DM_Sans]">
                  #{pair.ticketAId}
                </span>
                <span className="mt-0.5 block text-sm text-slate-800 leading-snug line-clamp-2">
                  {pair.ticketATitre}
                </span>
              </div>
              <div className="rounded-xl bg-slate-50 px-3 py-2">
                <span className="block text-xs font-semibold text-primary-500 font-[DM_Sans]">
                  #{pair.ticketBId}
                </span>
                <span className="mt-0.5 block text-sm text-slate-800 leading-snug line-clamp-2">
                  {pair.ticketBTitre}
                </span>
              </div>
            </div>
            <div className="space-y-1">
              <SimilarityBar value={pair.similarity} />
              {pair.groupe && (
                <p className="text-xs text-slate-400">
                  Groupe : <strong className="text-slate-800 font-[DM_Sans]">{pair.groupe}</strong>
                </p>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
