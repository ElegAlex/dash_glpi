import { useLocation } from "react-router";

const pageTitles: Record<string, string> = {
  "/import": "Import CSV",
  "/stock": "Tableau de bord — Stock",
  "/categories": "Catégories",
  "/bilan": "Bilan d'activité",
  "/mining": "Text Mining",
  "/export": "Export Excel",
  "/settings": "Paramètres",
};

function PageHeader() {
  const location = useLocation();
  const basePath = "/" + location.pathname.split("/")[1];
  const title = pageTitles[basePath] ?? "PILOTAGE GLPI";

  return (
    <header className="h-14 border-b border-gray-200 bg-white px-6 flex items-center">
      <h2 className="text-lg font-semibold text-gray-800">{title}</h2>
    </header>
  );
}

export default PageHeader;
