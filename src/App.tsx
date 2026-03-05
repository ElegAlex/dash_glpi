import { Routes, Route, Navigate } from "react-router";
import Layout from "./components/layout/Layout";
import DashboardPage from "./pages/DashboardPage";
import DataPage from "./pages/DataPage";
import StockPage from "./pages/StockPage";
import TechnicianDetail from "./pages/TechnicianDetail";
import CategoriesPage from "./pages/CategoriesPage";
import BilanPage from "./pages/BilanPage";
import DelaisPage from "./pages/DelaisPage";
import MiningPage from "./pages/MiningPage";
import SuiviPage from "./pages/SuiviPage";
import SettingsPage from "./pages/SettingsPage";

function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/data" element={<DataPage />} />
        <Route path="/import" element={<Navigate to="/data" replace />} />
        <Route path="/export" element={<Navigate to="/data" replace />} />
        <Route path="/stock" element={<StockPage />} />
        <Route path="/suivi" element={<SuiviPage />} />
        <Route path="/suivi/:technicien" element={<TechnicianDetail />} />
        <Route path="/categories" element={<CategoriesPage />} />
        <Route path="/bilan" element={<BilanPage />} />
        <Route path="/delais" element={<DelaisPage />} />
        <Route path="/mining" element={<MiningPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}

export default App;
