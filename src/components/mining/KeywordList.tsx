import type { KeywordFrequency } from "../../types/mining";

interface KeywordListProps {
  keywords: KeywordFrequency[];
  title: string;
  maxItems?: number;
  onKeywordClick?: (word: string) => void;
}

export default function KeywordList({
  keywords,
  title,
  maxItems = 20,
  onKeywordClick,
}: KeywordListProps) {
  const items = keywords.slice(0, maxItems);
  const maxScore = items.length > 0 ? Math.max(...items.map((k) => k.tfidfScore)) : 1;

  return (
    <div className="rounded-xl border border-[#e2e6ee] bg-white p-4 shadow-sm">
      <h3 className="mb-3 text-sm font-semibold text-[#1a1f2e]">{title}</h3>
      <ol className="space-y-1">
        {items.map((kw, i) => {
          const barWidth = maxScore > 0 ? (kw.tfidfScore / maxScore) * 100 : 0;
          return (
            <li key={kw.word} className="group">
              <button
                className="w-full rounded px-2 py-1 text-left transition-colors hover:bg-[#f1f3f7]"
                onClick={() => onKeywordClick?.(kw.word)}
                disabled={!onKeywordClick}
                style={{ cursor: onKeywordClick ? "pointer" : "default" }}
              >
                <div className="flex items-center gap-2">
                  <span className="w-6 shrink-0 text-right text-xs font-medium text-[#6e7891]">
                    #{i + 1}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-sm font-medium text-[#1a1f2e]">
                    {kw.word}
                  </span>
                  <span className="shrink-0 text-xs tabular-nums text-[#525d73]">
                    {kw.tfidfScore.toFixed(3)}
                  </span>
                </div>
                <div className="mt-0.5 ml-8 flex items-center gap-2">
                  <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-[#e2e6ee]">
                    <div
                      className="h-full rounded-full bg-[#0C419A]"
                      style={{ width: `${barWidth}%` }}
                    />
                  </div>
                  <span className="shrink-0 text-xs text-[#9ba8bc]">
                    {kw.docFrequency} doc{kw.docFrequency > 1 ? "s" : ""}
                  </span>
                </div>
              </button>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
