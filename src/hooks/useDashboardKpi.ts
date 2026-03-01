import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useInvoke } from './useInvoke';
import type { DashboardKpi } from '../types/dashboard';
import type { ImportHistory } from '../types/config';

export function useDashboardKpi() {
  const { data, loading, error, execute } = useInvoke<DashboardKpi>();
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    invoke<ImportHistory[]>('get_import_history').then((history) => {
      const active = history.find((h) => h.isActive);
      if (active) {
        execute('get_dashboard_kpi', {
          dateDebut: active.dateRangeFrom ?? undefined,
          dateFin: active.dateRangeTo ?? undefined,
        });
      }
    }).catch(() => {
      execute('get_dashboard_kpi', {});
    });
  }, [execute]);

  return { data, loading, error, reload: execute };
}
