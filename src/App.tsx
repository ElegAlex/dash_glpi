import { Routes, Route, Navigate } from "react-router";
import Layout from "./components/layout/Layout";
import DashboardPage from "./pages/DashboardPage";
import ImportPage from "./pages/ImportPage";
import StockPage from "./pages/StockPage";
import TechnicianDetail from "./pages/TechnicianDetail";
import CategoriesPage from "./pages/CategoriesPage";
import BilanPage from "./pages/BilanPage";
import MiningPage from "./pages/MiningPage";
import TimelineView from "./pages/TimelineView";
import ExportPage from "./pages/ExportPage";
import SettingsPage from "./pages/SettingsPage";

function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/import" element={<ImportPage />} />
        <Route path="/stock" element={<StockPage />} />
        <Route path="/stock/:technicien" element={<TechnicianDetail />} />
        <Route path="/categories" element={<CategoriesPage />} />
        <Route path="/bilan" element={<BilanPage />} />
        <Route path="/mining" element={<MiningPage />} />
        <Route path="/timeline" element={<TimelineView />} />
        <Route path="/export" element={<ExportPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}

export default App;
