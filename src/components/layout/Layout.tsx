import { Outlet } from "react-router";
import Sidebar from "./Sidebar";

function Layout() {
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
