import { X } from "lucide-react";
import type { TicketRef } from "../../types/mining";

interface DrillDownPanelProps {
  title: string;
  tickets: TicketRef[];
  onClose: () => void;
}

export default function DrillDownPanel({ title, tickets, onClose }: DrillDownPanelProps) {
  return (
    <div className="fixed inset-y-0 right-0 z-40 w-[480px] max-w-full bg-white shadow-[-4px_0_16px_rgba(0,0,0,0.08)] flex flex-col">
      <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
        <div>
          <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-800">{title}</h3>
          <p className="text-xs text-slate-400 mt-0.5">
            {tickets.length} ticket{tickets.length !== 1 ? "s" : ""}
          </p>
        </div>
        <button
          onClick={onClose}
          className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-colors"
        >
          <X size={18} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-5 py-3 space-y-2">
        {tickets.map((ticket) => (
          <div
            key={ticket.id}
            className="rounded-xl bg-slate-50 px-3 py-2 hover:bg-[rgba(12,65,154,0.04)] transition-colors"
          >
            <span className="block text-xs font-semibold text-primary-500 font-[DM_Sans]">
              #{ticket.id}
            </span>
            <span className="block text-sm text-slate-800 leading-snug mt-0.5">
              {ticket.titre}
            </span>
          </div>
        ))}
        {tickets.length === 0 && (
          <p className="py-8 text-center text-sm text-slate-400">Aucun ticket</p>
        )}
      </div>
    </div>
  );
}
