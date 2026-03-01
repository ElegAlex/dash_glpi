export interface TextAnalysisRequest {
  corpus: string;
  scope: string;
  groupBy?: string;
  topN?: number;
  includeResolved?: boolean;
}

export interface KeywordFrequency {
  word: string;
  count: number;
  tfidfScore: number;
  docFrequency: number;
}

export interface GroupKeywords {
  groupName: string;
  keywords: KeywordFrequency[];
  ticketCount: number;
}

export interface CorpusStats {
  totalDocuments: number;
  totalTokens: number;
  vocabularySize: number;
  avgTokensPerDoc: number;
}

export interface TextAnalysisResult {
  keywords: KeywordFrequency[];
  byGroup: GroupKeywords[] | null;
  corpusStats: CorpusStats;
  ticketMap: Record<string, TicketRef[]>;
}

// ── Clusters (US025) ──
export interface ClusterResult {
  clusters: ClusterInfo[];
  silhouetteScore: number;
  totalTickets: number;
}

export interface ClusterInfo {
  id: number;
  label: string;
  topKeywords: string[];
  ticketCount: number;
  ticketIds: number[];
  avgResolutionDays: number | null;
}

// ── Anomalies (US026) ──
export interface AnomalyAlert {
  ticketId: number;
  titre: string;
  anomalyType: string;
  severity: string;
  description: string;
  metricValue: number;
  expectedRange: string;
}

// ── Doublons (US024) ──
export interface DuplicatePair {
  ticketAId: number;
  ticketATitre: string;
  ticketBId: number;
  ticketBTitre: string;
  similarity: number;
  groupe: string;
}

// ── Co-occurrence Network ──
export interface CooccurrenceRequest {
  topNNodes?: number;
  maxEdges?: number;
  includeResolved?: boolean;
}

export interface CooccurrenceNode {
  id: string;
  tfidfScore: number;
  docFrequency: number;
}

export interface CooccurrenceEdge {
  source: string;
  target: string;
  weight: number;
}

export interface TicketRef {
  id: number;
  titre: string;
}

export interface CooccurrenceResult {
  nodes: CooccurrenceNode[];
  edges: CooccurrenceEdge[];
  ticketMap: Record<string, TicketRef[]>;
}
