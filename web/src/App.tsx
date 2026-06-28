import { useEffect, useState } from "react";
import { Navigate, Route, BrowserRouter as Router, Routes, useLocation } from "react-router-dom";
import { getSetupStatus } from "./api/setupApi";
import { ConfigurePage } from "./pages/ConfigurePage";
import { ReadyPage } from "./pages/ReadyPage";
import { UnlockPage } from "./pages/UnlockPage";

function SetupRoutes() {
  const location = useLocation();
  const [setupRequired, setSetupRequired] = useState<boolean | null>(null);

  useEffect(() => {
    getSetupStatus()
      .then((status) => setSetupRequired(status.setupRequired))
      .catch(() => setSetupRequired(false));
  }, [location.pathname]);

  if (setupRequired === null) {
    return <div className="app-loading" aria-hidden="true" />;
  }

  return (
    <Routes>
      <Route
        path="/"
        element={setupRequired ? <Navigate to="/setup" replace /> : <ReadyPage />}
      />
      <Route
        path="/setup"
        element={setupRequired ? <UnlockPage /> : <Navigate to="/" replace />}
      />
      <Route
        path="/setup/configure"
        element={setupRequired ? <ConfigurePage /> : <Navigate to="/" replace />}
      />
      <Route path="*" element={<Navigate to={setupRequired ? "/setup" : "/"} replace />} />
    </Routes>
  );
}

export function App() {
  return (
    <Router>
      <SetupRoutes />
    </Router>
  );
}
