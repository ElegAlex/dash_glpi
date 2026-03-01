import { ExportPanel } from '../components/shared/ExportPanel';

function ExportPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-[#1a1f2e]">Exports Excel</h1>
        <p className="mt-0.5 text-sm text-[#525d73]">
          Générez des fichiers Excel et ZIP prêts à diffuser depuis les données du dernier import.
        </p>
      </div>
      <ExportPanel />
    </div>
  );
}

export default ExportPage;
