import { ReactElement, useEffect, useState } from "react";
import { Navigate, Route, BrowserRouter as Router, Routes, useLocation } from "react-router-dom";
import { AuthUser, getCurrentUser, logout } from "./api/authApi";
import { getSetupStatus } from "./api/setupApi";
import { AdminLayout } from "./components/AdminLayout";
import { UploadProgressProvider } from "./components/UploadProgress";
import { BucketsPage } from "./pages/BucketsPage";
import { ConfigurePage } from "./pages/ConfigurePage";
import { DashboardPage } from "./pages/DashboardPage";
import { LoginPage } from "./pages/LoginPage";
import { MetricsPage } from "./pages/MetricsPage";
import { ObjectsPage } from "./pages/ObjectsPage";
import { ReplicasPage } from "./pages/ReplicasPage";
import { SettingsPage } from "./pages/SettingsPage";
import { UnlockPage } from "./pages/UnlockPage";

function SetupRoutes() {
  const location = useLocation();
  const [setupRequired, setSetupRequired] = useState<boolean | null>(null);
  const [serverVersion, setServerVersion] = useState<string | null>(null);
  const [internalWebPort, setInternalWebPort] = useState(8080);
  const [internalS3Port, setInternalS3Port] = useState(9000);
  const [publicWebUrl, setPublicWebUrl] = useState<string | null>(null);
  const [publicS3Url, setPublicS3Url] = useState<string | null>(null);
  const [user, setUser] = useState<AuthUser | null>(null);
  const [authLoaded, setAuthLoaded] = useState(false);

  useEffect(() => {
    getSetupStatus()
      .then((status) => {
        setSetupRequired(status.setupRequired);
        setServerVersion(status.serverVersion);
        setInternalWebPort(status.internalWebPort);
        setInternalS3Port(status.internalS3Port);
        setPublicWebUrl(status.publicWebUrl);
        setPublicS3Url(status.publicS3Url);
      })
      .catch(() => {
        setSetupRequired(false);
        setServerVersion(null);
      });
  }, [location.pathname]);

  useEffect(() => {
    if (setupRequired === null) {
      setUser(null);
      setAuthLoaded(false);
      return;
    }

    if (setupRequired !== false) {
      setUser(null);
      setAuthLoaded(true);
      return;
    }

    setAuthLoaded(false);
    getCurrentUser()
      .then(setUser)
      .catch(() => setUser({ authenticated: false, username: null }))
      .finally(() => setAuthLoaded(true));
  }, [setupRequired]);

  async function handleLogout() {
    await logout().catch(() => undefined);
    setUser({ authenticated: false, username: null });
  }

  if (setupRequired === null || !authLoaded) {
    return <div className="app-loading" aria-hidden="true" />;
  }

  const authenticated = user?.authenticated === true;
  const adminElement = (children: ReactElement) => authenticated ? (
    <AdminLayout username={user.username} onLogout={handleLogout}>
      {children}
    </AdminLayout>
  ) : (
    <Navigate to="/login" replace />
  );

  return (
    <Routes>
      <Route
        path="/"
        element={
          setupRequired ? (
            <Navigate to="/setup" replace />
          ) : (
            <Navigate to={authenticated ? "/dashboard" : "/login"} replace />
          )
        }
      />
      <Route
        path="/setup"
        element={setupRequired ? <UnlockPage serverVersion={serverVersion} /> : <Navigate to="/" replace />}
      />
      <Route
        path="/setup/configure"
        element={setupRequired ? (
          <ConfigurePage
            serverVersion={serverVersion}
            internalWebPort={internalWebPort}
            internalS3Port={internalS3Port}
            configuredPublicWebUrl={publicWebUrl}
            configuredPublicS3Url={publicS3Url}
          />
        ) : <Navigate to="/" replace />}
      />
      <Route
        path="/login"
        element={
          setupRequired ? (
            <Navigate to="/setup" replace />
          ) : authenticated ? (
            <Navigate to="/dashboard" replace />
          ) : (
            <LoginPage onAuthenticated={setUser} />
          )
        }
      />
      <Route path="/dashboard" element={adminElement(<DashboardPage />)} />
      <Route path="/buckets" element={adminElement(<BucketsPage />)} />
      <Route path="/objects" element={adminElement(<ObjectsPage />)} />
      <Route path="/replicas" element={adminElement(<ReplicasPage />)} />
      <Route path="/metrics" element={adminElement(<MetricsPage />)} />
      <Route path="/settings" element={adminElement(<SettingsPage />)} />
      <Route path="*" element={<Navigate to={setupRequired ? "/setup" : "/"} replace />} />
    </Routes>
  );
}

export function App() {
  return (
    <UploadProgressProvider>
      <Router>
        <SetupRoutes />
      </Router>
    </UploadProgressProvider>
  );
}
