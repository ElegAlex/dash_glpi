import { useState } from "react";
import Wordcloud from "@visx/wordcloud/lib/Wordcloud";
import { scaleLog } from "@visx/scale";
import { Text } from "@visx/text";
import type { KeywordFrequency } from "../../types/mining";

const CHART_PALETTE = [
  "#0C419A",
  "#E69F00",
  "#009E73",
  "#D55E00",
  "#56B4E9",
  "#CC79A7",
  "#0072B2",
  "#228833",
];

interface WordDatum {
  text: string;
  value: number;
  tfidfScore: number;
}

interface WordCloudProps {
  keywords: KeywordFrequency[];
  width?: number;
  height?: number;
  onWordClick?: (word: string) => void;
}

export default function WordCloud({
  keywords,
  width = 800,
  height = 400,
  onWordClick,
}: WordCloudProps) {
  const [hoveredWord, setHoveredWord] = useState<string | null>(null);
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    word: string;
    score: number;
  } | null>(null);

  const top50 = keywords.slice(0, 50);

  const words: WordDatum[] = top50.map((kw) => ({
    text: kw.word,
    value: kw.tfidfScore,
    tfidfScore: kw.tfidfScore,
  }));

  const minScore = Math.min(...words.map((w) => w.value));
  const maxScore = Math.max(...words.map((w) => w.value));

  const fontScale = scaleLog({
    domain: [Math.max(minScore, 0.0001), Math.max(maxScore, 0.001)],
    range: [12, 60],
  });

  const getColor = (index: number) => CHART_PALETTE[index % CHART_PALETTE.length];

  if (words.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-sm text-[#6e7891]"
        style={{ width, height }}
      >
        Aucun mot-clé à afficher
      </div>
    );
  }

  return (
    <div className="relative" style={{ width, height }}>
      <svg width={width} height={height}>
        <Wordcloud
          words={words}
          width={width}
          height={height}
          fontSize={(datum) => fontScale(datum.value)}
          font="Inter, system-ui, sans-serif"
          fontWeight={600}
          padding={4}
          rotate={0}
          random={() => 0.5}
        >
          {(cloudWords) =>
            cloudWords.map((w, i) => {
              const isHovered = hoveredWord === w.text;
              const opacity =
                hoveredWord === null ? 1 : isHovered ? 1 : 0.4;
              return (
                <Text
                  key={w.text}
                  fill={getColor(i)}
                  textAnchor="middle"
                  transform={`translate(${(w.x ?? 0) + width / 2}, ${(w.y ?? 0) + height / 2}) rotate(${w.rotate ?? 0})`}
                  fontSize={w.size}
                  fontFamily={w.font}
                  fontWeight={600}
                  style={{
                    opacity,
                    cursor: onWordClick ? "pointer" : "default",
                    transition: "opacity 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    setHoveredWord(w.text ?? null);
                    const rect = (
                      e.currentTarget as SVGTextElement
                    ).getBoundingClientRect();
                    setTooltip({
                      x: rect.left + rect.width / 2,
                      y: rect.top - 8,
                      word: w.text ?? "",
                      score: (w as unknown as WordDatum).tfidfScore ?? 0,
                    });
                  }}
                  onMouseLeave={() => {
                    setHoveredWord(null);
                    setTooltip(null);
                  }}
                  onClick={() => {
                    if (onWordClick && w.text) onWordClick(w.text);
                  }}
                >
                  {w.text}
                </Text>
              );
            })
          }
        </Wordcloud>
      </svg>

      {tooltip && (
        <div
          className="pointer-events-none fixed z-50 rounded-md bg-[#1a1f2e] px-2 py-1 text-xs text-white shadow-lg"
          style={{ left: tooltip.x, top: tooltip.y, transform: "translateX(-50%) translateY(-100%)" }}
        >
          <span className="font-semibold">{tooltip.word}</span>
          <span className="ml-2 text-[#9ba8bc]">TF-IDF: {tooltip.score.toFixed(4)}</span>
        </div>
      )}
    </div>
  );
}
