import { useDashboardKpi } from '../hooks/useDashboardKpi';
import { KpiCards } from '../components/dashboard/KpiCards';
import { VolumeChart } from '../components/dashboard/VolumeChart';
import { MttrTrendChart } from '../components/dashboard/MttrTrendChart';
import { DistributionDelaisChart } from '../components/dashboard/DistributionDelaisChart';
import { TauxN1TrendChart } from '../components/dashboard/TauxN1TrendChart';
import { TypologieSection } from '../components/dashboard/TypologieSection';
import { MttrComparatifTable } from '../components/dashboard/MttrComparatifTable';
import { Card } from '../components/shared/Card';

function DashboardPage() {
  const { data, loading, error } = useDashboardKpi();

  const subtitle = data
    ? `${data.meta.totalTickets.toLocaleString('fr-FR')} tickets · ${data.meta.plageDates[0]} au ${data.meta.plageDates[1]} · Calcul en ${data.meta.calculDurationMs}ms`
    : null;

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Dashboard ITSM
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          {subtitle ?? 'Vue synthetique des indicateurs ITSM'}
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
            {error}
          </div>
        )}

        {loading ? (
          <div className="py-12 text-center text-sm text-slate-400">
            Chargement du dashboard...
          </div>
        ) : !data ? (
          <div className="py-12 text-center text-sm text-slate-400">
            Importez un fichier CSV pour afficher le dashboard.
          </div>
        ) : (
          <>
            {/* KPI Cards */}
            <div className="animate-fade-slide-up">
              <KpiCards kpi={data} />
            </div>

            {/* Volume + MTTR Trend */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 animate-fade-slide-up animation-delay-150">
              <Card>
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                  Volumetrie mensuelle
                </h2>
                <VolumeChart data={data.volumes.parMois} />
              </Card>
              <Card>
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                  Tendance MTTR
                </h2>
                <MttrTrendChart data={data.resolution.trendMensuel} />
              </Card>
            </div>

            {/* Taux N1 Trend + Distribution Delais */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 animate-fade-slide-up animation-delay-300">
              <Card>
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                  Tendance taux N1
                </h2>
                <TauxN1TrendChart
                  data={data.tauxN1.trendMensuel}
                  objectifItil={data.tauxN1.objectifItil}
                />
              </Card>
              <Card>
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                  Distribution des delais de resolution
                </h2>
                <DistributionDelaisChart data={data.resolution.distributionTranches} />
              </Card>
            </div>

            {/* Typologie */}
            <div className="animate-fade-slide-up animation-delay-450">
              <TypologieSection typologie={data.typologie} />
            </div>

            {/* MTTR Comparatif Table */}
            <div className="animate-fade-slide-up animation-delay-450">
              <Card>
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                  MTTR par technicien
                </h2>
                <MttrComparatifTable data={data.resolution.parTechnicien} />
              </Card>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export default DashboardPage;
