import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { useInvoke } from '../hooks/useInvoke';

interface TechnicianSummary {
  technicien: string;
  totalTraites: number;
  stockActuel: number;
  incidents: number;
  demandes: number;
  couleurSeuil: string;
}

const RAG_STYLES: Record<string, string> = {
  vert: 'bg-success-50 text-success-500',
  jaune: 'bg-accent-50 text-accent-700',
  orange: 'bg-warning-50 text-warning-500',
  rouge: 'bg-danger-50 text-danger-500',
};

function SuiviPage() {
  const navigate = useNavigate();
  const { data, loading, error, execute } = useInvoke<TechnicianSummary[]>();
  const [search, setSearch] = useState('');

  useEffect(() => {
    execute('get_all_technicians', {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const filtered = useMemo(() => {
    if (!data) return [];
    const q = search.toLowerCase().trim();
    const list = q ? data.filter((t) => t.technicien.toLowerCase().includes(q)) : data;
    return [...list].sort((a, b) => a.technicien.localeCompare(b.technicien, 'fr'));
  }, [data, search]);

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-3 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Suivi individuel
        </h1>
        <p className="text-sm text-slate-400 mt-0.5">
          {data ? `${data.length} technicien(s) actif(s)` : 'Historique et KPI par technicien'}
        </p>
        <div className="mt-3">
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Rechercher un technicien..."
            className="w-full max-w-md px-4 py-2 rounded-xl bg-white text-sm text-slate-800
              shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
              placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-primary-500/30
              font-[Source_Sans_3]"
          />
        </div>
      </header>

      <div className="px-8 pb-8 pt-6">
        {error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500 mb-6">
            {error}
          </div>
        )}

        {loading && !data && (
          <div className="py-12 text-center text-sm text-slate-400">Chargement...</div>
        )}

        {data && filtered.length === 0 && (
          <div className="py-12 text-center text-sm text-slate-400">
            Aucun technicien trouve
          </div>
        )}

        {filtered.length > 0 && (
          <div className="space-y-2 animate-fade-slide-up">
            {filtered.map((tech) => (
              <div
                key={tech.technicien}
                onClick={() => navigate(`/suivi/${encodeURIComponent(tech.technicien)}`)}
                className="bg-white rounded-2xl px-6 py-4 cursor-pointer group
                  shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
                  hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]
                  transition-all duration-200 flex items-center gap-6"
              >
                <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-800 group-hover:text-primary-500 transition-colors truncate min-w-[200px]">
                  {tech.technicien}
                </h3>
                <span className={`shrink-0 px-2.5 py-0.5 rounded-lg text-xs font-semibold capitalize ${RAG_STYLES[tech.couleurSeuil] ?? ''}`}>
                  {tech.stockActuel} en stock
                </span>
                <div className="flex items-center gap-5 text-xs text-slate-400 font-[Source_Sans_3] ml-auto">
                  <span>{tech.totalTraites} traites</span>
                  <span>{tech.incidents} inc.</span>
                  <span>{tech.demandes} dem.</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default SuiviPage;
