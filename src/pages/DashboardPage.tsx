import { useState, useEffect, useCallback, useRef } from 'react';
import { subDays } from 'date-fns';
import { invoke } from '@tauri-apps/api/core';
import { useInvoke } from '../hooks/useInvoke';
import { CompactFilterBar } from '../components/shared/CompactFilterBar';
import { KpiCards } from '../components/dashboard/KpiCards';
import { VolumeChart } from '../components/dashboard/VolumeChart';
import { MttrTrendChart } from '../components/dashboard/MttrTrendChart';
import { DistributionDelaisChart } from '../components/dashboard/DistributionDelaisChart';
import { TauxN1TrendChart } from '../components/dashboard/TauxN1TrendChart';
import { TypologieSection } from '../components/dashboard/TypologieSection';
import { MttrComparatifTable } from '../components/dashboard/MttrComparatifTable';
import { Card } from '../components/shared/Card';
import type { DashboardKpi } from '../types/dashboard';
import type { ImportHistory } from '../types/config';
import { type DateRange, type Granularity } from '../components/shared/DateRangePicker';

function formatDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function DashboardPage() {
  const today = new Date();
  const [range, setRange] = useState<DateRange>({ from: subDays(today, 29), to: today });
  const [granularity, setGranularity] = useState<Granularity>('month');
  const { data, loading, error, execute } = useInvoke<DashboardKpi>();
  const initialized = useRef(false);

  const load = useCallback(
    (r: DateRange, g: Granularity) => {
      execute('get_dashboard_kpi', {
        dateDebut: formatDate(r.from),
        dateFin: formatDate(r.to),
        granularity: g,
      });
    },
    [execute],
  );

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    invoke<ImportHistory[]>('get_import_history')
      .then((history) => {
        const active = history.find((h) => h.isActive);
        if (active?.dateRangeFrom && active?.dateRangeTo) {
          const from = new Date(active.dateRangeFrom);
          const to = new Date(active.dateRangeTo);
          setRange({ from, to });
          const days = (to.getTime() - from.getTime()) / 86400000;
          const g: Granularity = days > 365 ? 'quarter' : days > 60 ? 'month' : days > 14 ? 'week' : 'day';
          setGranularity(g);
          load({ from, to }, g);
        } else {
          load(range, granularity);
        }
      })
      .catch(() => {
        load(range, granularity);
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleRangeChange = (r: DateRange, autoGran: Granularity) => {
    setRange(r);
    setGranularity(autoGran);
    load(r, autoGran);
  };

  const handleGranularityChange = (g: Granularity) => {
    setGranularity(g);
    load(range, g);
  };

  const subtitle = data
    ? `${data.meta.totalTickets.toLocaleString('fr-FR')} tickets · Calcul en ${data.meta.calculDurationMs}ms`
    : null;

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-3 border-b border-slate-200/30">
        <div className="flex items-baseline justify-between">
          <div>
            <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
              Dashboard ITSM
            </h1>
            <p className="text-sm text-slate-400 mt-0.5">
              {subtitle ?? 'Vue synthetique des indicateurs ITSM'}
            </p>
          </div>
        </div>
        <CompactFilterBar
          range={range}
          granularity={granularity}
          onRangeChange={handleRangeChange}
          onGranularityChange={handleGranularityChange}
        />
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
              <KpiCards kpi={data} granularity={granularity} />
            </div>

            {/* Volume + MTTR Trend */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 animate-fade-slide-up animation-delay-150">
              <Card>
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                  Volumetrie {granularity === 'day' ? 'journaliere' : granularity === 'week' ? 'hebdomadaire' : granularity === 'quarter' ? 'trimestrielle' : 'mensuelle'}
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
              <TypologieSection typologie={data.typologie} volumes={data.volumes.parMois} />
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
