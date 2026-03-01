import { useInvoke } from './useInvoke';
import type { DashboardKpi } from '../types/dashboard';

export function useDashboardKpi() {
  return useInvoke<DashboardKpi>();
}
