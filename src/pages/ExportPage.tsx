import { ExportPanel } from '../components/shared/ExportPanel';

function ExportPage() {
  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Exports Excel
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Generez des fichiers Excel et ZIP prets a diffuser depuis les donnees du dernier import
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        <ExportPanel />
      </div>
    </div>
  );
}

export default ExportPage;
