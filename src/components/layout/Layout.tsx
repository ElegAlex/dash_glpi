import { useEffect } from "react";
import { Outlet } from "react-router";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./Sidebar";
import { useSettingsStore } from "../../stores/settingsStore";
import type { AppConfig } from "../../types/config";

function Layout() {
  const setConfig = useSettingsStore((s) => s.setConfig);

  useEffect(() => {
    invoke<AppConfig>("get_config").then(setConfig).catch(() => {});
  }, []);

  return (
    <div className="flex h-screen bg-[#F5F7FA]">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  );
}

export default Layout;
