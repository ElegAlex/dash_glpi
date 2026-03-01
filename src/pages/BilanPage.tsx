import { useState, useEffect, useCallback, useRef } from 'react';
import { subDays } from 'date-fns';
import { invoke } from '@tauri-apps/api/core';
import { ArrowDownToLine, ArrowUpFromLine, TrendingUp, BarChart3 } from 'lucide-react';
import { PeriodSelector } from '../components/bilan/PeriodSelector';
import { BilanChart } from '../components/bilan/BilanChart';
import { KpiCard } from '../components/shared/KpiCard';
import { BilanTable } from '../components/shared/BilanTable';
import { Card } from '../components/shared/Card';
import { useInvoke } from '../hooks/useInvoke';
import type { BilanTemporel } from '../types/kpi';
import type { ImportHistory } from '../types/config';
import { type DateRange, type Granularity } from '../components/shared/DateRangePicker';

function formatDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function BilanPage() {
  const today = new Date();
  const [range, setRange] = useState<DateRange>({ from: subDays(today, 29), to: today });
  const [granularity, setGranularity] = useState<Granularity>('month');
  const { data, loading, error, execute } = useInvoke<BilanTemporel>();
  const initialized = useRef(false);

  const load = useCallback(
    (r: DateRange, g: Granularity) => {
      execute('get_bilan_temporel', {
        request: {
          period: g,
          dateFrom: formatDate(r.from),
          dateTo: formatDate(r.to),
        },
      });
    },
    [execute],
  );

  useEffect(() => {
    invoke<ImportHistory[]>('get_import_history').then((history) => {
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
      initialized.current = true;
    }).catch(() => {
      load(range, granularity);
      initialized.current = true;
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

  const totaux = data?.totaux;
  const periodes = data?.periodes ?? [];

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Bilan d'activite
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Flux entrants et sortants sur la periode selectionnee
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        <PeriodSelector
          range={range}
          granularity={granularity}
          onRangeChange={handleRangeChange}
          onGranularityChange={handleGranularityChange}
        />

        {error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
            {error}
          </div>
        )}

        {loading ? (
          <div className="py-12 text-center text-sm text-slate-400">Chargement du bilan...</div>
        ) : (
          <>
            {/* KPI Cards */}
            {totaux && (
              <div className="grid grid-cols-4 gap-5 animate-fade-slide-up">
                <KpiCard
                  label="Total entrants"
                  value={totaux.totalEntrees}
                  icon={<ArrowDownToLine size={18} color="#1565C0" />}
                  accentColor="#1565C0"
                />
                <KpiCard
                  label="Total sortants"
                  value={totaux.totalSorties}
                  icon={<ArrowUpFromLine size={18} color="#2E7D32" />}
                  accentColor="#2E7D32"
                />
                <KpiCard
                  label="Delta global"
                  value={totaux.deltaGlobal}
                  trend={totaux.deltaGlobal > 0 ? 'up' : totaux.deltaGlobal < 0 ? 'down' : 'neutral'}
                  trendIsGood={false}
                  icon={<TrendingUp size={18} color="#FF8F00" />}
                  accentColor="#FF8F00"
                />
                <KpiCard
                  label="Moy. / periode"
                  value={Number(totaux.moyenneEntreesParPeriode.toFixed(1))}
                  format="number"
                  icon={<BarChart3 size={18} color="#0C419A" />}
                  accentColor="#0C419A"
                />
              </div>
            )}

            {/* Chart */}
            {periodes.length > 0 && (
              <div className="animate-fade-slide-up animation-delay-150">
                <Card>
                  <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-4">
                    Evolution des flux
                  </h2>
                  <BilanChart periodes={periodes} />
                </Card>
              </div>
            )}

            {/* BilanTable */}
            {totaux && (
              <div className="animate-fade-slide-up animation-delay-300">
                <BilanTable rows={[
                  { label: 'Total entrants', value: totaux.totalEntrees },
                  { label: 'Total sortants', value: totaux.totalSorties },
                  {
                    label: 'Delta global',
                    value: totaux.deltaGlobal > 0
                      ? `+${totaux.deltaGlobal.toLocaleString('fr-FR')}`
                      : totaux.deltaGlobal.toLocaleString('fr-FR'),
                    highlight: true,
                    negative: totaux.deltaGlobal > 0,
                    positive: totaux.deltaGlobal < 0,
                  },
                  {
                    label: 'Moy. entrants / periode',
                    value: totaux.moyenneEntreesParPeriode.toLocaleString('fr-FR', {
                      maximumFractionDigits: 1,
                    }),
                  },
                  {
                    label: 'Moy. sortants / periode',
                    value: totaux.moyenneSortiesParPeriode.toLocaleString('fr-FR', {
                      maximumFractionDigits: 1,
                    }),
                  },
                ]} />
              </div>
            )}

            {!data && !loading && !error && (
              <div className="py-12 text-center text-sm text-slate-400">
                Selectionnez une periode pour afficher le bilan.
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default BilanPage;
