import { Link, useLocation } from "react-router";
import {
  LayoutDashboard,
  Upload,
  BarChart3,
  FolderTree,
  TrendingUp,
  Search,
  GitBranch,
  Download,
  Settings,
} from "lucide-react";

const NAV_ITEMS = [
  { icon: LayoutDashboard, label: "Dashboard", path: "/dashboard" },
  { icon: Upload, label: "Import", path: "/import" },
  { icon: BarChart3, label: "Stock", path: "/stock" },
  { icon: FolderTree, label: "Categories", path: "/categories" },
  { icon: TrendingUp, label: "Bilan", path: "/bilan" },
  { icon: Search, label: "Text Mining", path: "/mining" },
  { icon: GitBranch, label: "Longitudinal", path: "/timeline" },
  { icon: Download, label: "Export", path: "/export" },
  { icon: Settings, label: "Parametres", path: "/settings" },
] as const;

export default function Sidebar() {
  const { pathname } = useLocation();
  const basePath = "/" + pathname.split("/")[1];

  return (
    <aside
      className="
        w-60 min-h-screen flex-shrink-0 flex flex-col z-20
        bg-gradient-to-b from-[#0C419A] to-[#082A66]
        shadow-[0_20px_40px_rgba(0,0,0,0.14),0_8px_20px_rgba(0,0,0,0.10)]
      "
    >
      {/* Logo */}
      <div className="px-5 pt-7 pb-8">
        <h1 className="text-base font-bold text-white tracking-tight font-[DM_Sans]">
          PILOTAGE GLPI
        </h1>
        <p className="text-[10px] text-white/45 mt-0.5 tracking-[0.1em] uppercase">
          DSI CPAM 92
        </p>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 space-y-0.5">
        {NAV_ITEMS.map(({ icon: Icon, label, path }) => {
          const active = basePath === path;
          return (
            <Link
              key={path}
              to={path}
              className={`
                flex items-center gap-3 px-3 py-2.5 rounded-xl
                text-sm font-medium transition-all duration-150
                ${
                  active
                    ? "bg-white/15 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]"
                    : "text-white/65 hover:bg-white/8 hover:text-white"
                }
              `}
            >
              <Icon size={18} strokeWidth={active ? 2.2 : 1.8} />
              {label}
              {active && (
                <div
                  className="ml-auto w-1.5 h-1.5 rounded-full bg-amber-400
                    shadow-[0_0_8px_rgba(255,202,40,0.5)]"
                />
              )}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="px-5 py-4 border-t border-white/8">
        <p className="text-[10px] text-white/25">v1.0.0 · Tauri 2.10</p>
      </div>
    </aside>
  );
}
