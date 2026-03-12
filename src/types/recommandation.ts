export interface TechnicianSuggestion {
    technicien: string;
    scoreFinal: number;
    scoreCompetence: number;
    scoreCategorie: number;
    scoreTfidf: number;
    stockActuel: number;
    facteurCharge: number;
}

export interface AssignmentRecommendation {
    ticketId: number;
    ticketTitre: string;
    ticketCategorie: string | null;
    suggestions: TechnicianSuggestion[];
}

export interface ProfilingResult {
    profilesCount: number;
    vocabularySize: number;
    nbTicketsAnalysed: number;
    periodeFrom: string;
    periodeTo: string;
}

export interface RecommendationRequest {
    limitPerTicket?: number;
    scoreMinimum?: number;
}

export interface UnassignedTicketStats {
    count: number;
    ageMoyenJours: number;
}
