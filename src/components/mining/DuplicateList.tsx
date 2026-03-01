import type { DuplicatePair } from "../../types/mining";

interface DuplicateListProps {
  duplicates: DuplicatePair[];
}

function SimilarityBar({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  const color = pct >= 90 ? "#ce0500" : pct >= 75 ? "#E69F00" : "#0C419A";
  return (
    <div className="flex items-center gap-2">
      <div className="h-2 flex-1 overflow-hidden rounded-full bg-[#e2e6ee]">
        <div
          className="h-2 rounded-full transition-all"
          style={{ width: `${pct}%`, backgroundColor: color }}
        />
      </div>
      <span
        className="shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold"
        style={{ backgroundColor: `${color}18`, color }}
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
      <p className="text-sm text-[#6e7891]">
        <strong className="text-[#1a1f2e]">{duplicates.length}</strong> doublon
        {duplicates.length !== 1 ? "s" : ""} potentiel{duplicates.length !== 1 ? "s" : ""} détecté
        {duplicates.length !== 1 ? "s" : ""}
      </p>

      {sorted.length === 0 && (
        <div className="rounded-xl border border-dashed border-[#cdd3df] bg-white py-10 text-center text-sm text-[#6e7891]">
          Aucun doublon potentiel détecté
        </div>
      )}

      <div className="space-y-3">
        {sorted.map((pair) => (
          <div
            key={`${pair.ticketAId}-${pair.ticketBId}`}
            className="rounded-xl border border-[#e2e6ee] bg-white p-4 shadow-sm"
          >
            <div className="mb-3 grid grid-cols-2 gap-3">
              <div className="rounded-lg border border-[#e2e6ee] bg-[#f8f9fb] px-3 py-2">
                <span className="block text-xs font-semibold text-[#0C419A]">
                  #{pair.ticketAId}
                </span>
                <span className="mt-0.5 block text-sm text-[#1a1f2e] leading-snug line-clamp-2">
                  {pair.ticketATitre}
                </span>
              </div>
              <div className="rounded-lg border border-[#e2e6ee] bg-[#f8f9fb] px-3 py-2">
                <span className="block text-xs font-semibold text-[#0C419A]">
                  #{pair.ticketBId}
                </span>
                <span className="mt-0.5 block text-sm text-[#1a1f2e] leading-snug line-clamp-2">
                  {pair.ticketBTitre}
                </span>
              </div>
            </div>
            <div className="space-y-1">
              <SimilarityBar value={pair.similarity} />
              {pair.groupe && (
                <p className="text-xs text-[#6e7891]">
                  Groupe : <strong className="text-[#1a1f2e]">{pair.groupe}</strong>
                </p>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
