import { useState, useEffect, useCallback } from 'react';
import { subDays } from 'date-fns';
import { PeriodSelector } from '../components/bilan/PeriodSelector';
import { BilanChart } from '../components/bilan/BilanChart';
import { useInvoke } from '../hooks/useInvoke';
import type { BilanTemporel } from '../types/kpi';
import { type DateRange, type Granularity } from '../components/shared/DateRangePicker';

function formatDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function BilanPage() {
  const today = new Date();
  const [range, setRange] = useState<DateRange>({ from: subDays(today, 29), to: today });
  const [granularity, setGranularity] = useState<Granularity>('month');
  const { data, loading, error, execute } = useInvoke<BilanTemporel>();

  const load = useCallback(
    (r: DateRange, g: Granularity) => {
      execute('get_bilan_temporel', {
        period: g,
        dateFrom: formatDate(r.from),
        dateTo: formatDate(r.to),
      });
    },
    [execute],
  );

  useEffect(() => {
    load(range, granularity);
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
    <div className="space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-[#1a1f2e]">Bilan d'activité</h1>
        <p className="mt-0.5 text-sm text-[#525d73]">
          Flux entrants et sortants sur la période sélectionnée
        </p>
      </div>

      <PeriodSelector
        range={range}
        granularity={granularity}
        onRangeChange={handleRangeChange}
        onGranularityChange={handleGranularityChange}
      />

      {error && (
        <div className="rounded-md bg-[#fef2f2] border border-[#ce0500] px-4 py-3 text-sm text-[#af0400]">
          {error}
        </div>
      )}

      {loading ? (
        <div className="py-12 text-center text-sm text-[#6e7891]">Chargement du bilan…</div>
      ) : (
        <>
          {periodes.length > 0 && (
            <div className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)] p-4">
              <h2 className="text-sm font-semibold text-[#1a1f2e] mb-3">Évolution des flux</h2>
              <BilanChart periodes={periodes} />
            </div>
          )}

          {totaux && (
            <div className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)] overflow-hidden">
              <table className="w-full text-sm">
                <thead className="bg-[#f1f3f7]">
                  <tr>
                    {['Indicateur', 'Valeur'].map((h) => (
                      <th
                        key={h}
                        className="px-4 py-3 text-left text-xs font-medium text-[#525d73] uppercase tracking-wide"
                      >
                        {h}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#e2e6ee]">
                  {(
                    [
                      ['Total entrants', totaux.totalEntrees.toLocaleString('fr-FR'), false],
                      ['Total sortants', totaux.totalSorties.toLocaleString('fr-FR'), false],
                      [
                        'Delta global',
                        totaux.deltaGlobal > 0
                          ? `+${totaux.deltaGlobal.toLocaleString('fr-FR')}`
                          : totaux.deltaGlobal.toLocaleString('fr-FR'),
                        totaux.deltaGlobal > 0,
                      ],
                      [
                        'Moy. entrants / période',
                        totaux.moyenneEntreesParPeriode.toLocaleString('fr-FR', {
                          maximumFractionDigits: 1,
                        }),
                        false,
                      ],
                      [
                        'Moy. sortants / période',
                        totaux.moyenneSortiesParPeriode.toLocaleString('fr-FR', {
                          maximumFractionDigits: 1,
                        }),
                        false,
                      ],
                    ] as [string, string, boolean][]
                  ).map(([label, value, isDanger]) => (
                    <tr key={label} className="hover:bg-[#f8f9fb] transition-colors">
                      <td className="px-4 py-2.5 text-[#525d73]">{label}</td>
                      <td
                        className={`px-4 py-2.5 font-medium ${isDanger ? 'text-[#ce0500]' : 'text-[#1a1f2e]'}`}
                      >
                        {value}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {!data && !loading && !error && (
            <div className="py-12 text-center text-sm text-[#6e7891]">
              Sélectionnez une période pour afficher le bilan.
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default BilanPage;
