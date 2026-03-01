import { NavLink } from "react-router";
import {
  Upload,
  BarChart3,
  FolderTree,
  TrendingUp,
  Search,
  GitCompareArrows,
  FileSpreadsheet,
  Settings,
} from "lucide-react";

const navItems = [
  { to: "/import", label: "Import", icon: Upload },
  { to: "/stock", label: "Stock", icon: BarChart3 },
  { to: "/categories", label: "Catégories", icon: FolderTree },
  { to: "/bilan", label: "Bilan", icon: TrendingUp },
  { to: "/mining", label: "Text Mining", icon: Search },
  { to: "/timeline", label: "Longitudinal", icon: GitCompareArrows },
  { to: "/export", label: "Export", icon: FileSpreadsheet },
  { to: "/settings", label: "Paramètres", icon: Settings },
];

function Sidebar() {
  return (
    <aside className="w-60 bg-cpam-primary text-white flex flex-col">
      <div className="p-4 border-b border-white/20">
        <h1 className="text-lg font-bold">PILOTAGE GLPI</h1>
        <p className="text-xs text-white/60">DSI CPAM 92</p>
      </div>
      <nav className="flex-1 py-2">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
                isActive
                  ? "bg-white/20 text-white font-medium"
                  : "text-white/70 hover:bg-white/10 hover:text-white"
              }`
            }
          >
            <Icon size={18} />
            {label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}

export default Sidebar;
