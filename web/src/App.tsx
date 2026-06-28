import { useEffect, useState } from "react";
import { getSetupStatus } from "./api/setupApi";
import { ConfigurePage } from "./pages/ConfigurePage";
import { ReadyPage } from "./pages/ReadyPage";
import { UnlockPage } from "./pages/UnlockPage";

type Route = "/" | "/setup" | "/setup/configure";

function currentRoute(): Route {
  if (window.location.pathname === "/setup") {
    return "/setup";
  }

  if (window.location.pathname === "/setup/configure") {
    return "/setup/configure";
  }

  return "/";
}

function navigate(path: Route) {
  window.history.pushState({}, "", path);
  window.dispatchEvent(new PopStateEvent("popstate"));
}

export function App() {
  const [route, setRoute] = useState<Route>(currentRoute());
  const [setupRequired, setSetupRequired] = useState<boolean | null>(null);

  useEffect(() => {
    const updateRoute = () => setRoute(currentRoute());
    window.addEventListener("popstate", updateRoute);
    return () => window.removeEventListener("popstate", updateRoute);
  }, []);

  useEffect(() => {
    getSetupStatus()
      .then((status) => setSetupRequired(status.setupRequired))
      .catch(() => setSetupRequired(false));
  }, [route]);

  useEffect(() => {
    if (setupRequired === null) {
      return;
    }

    if (setupRequired && route === "/") {
      navigate("/setup");
    }

    if (!setupRequired && route !== "/") {
      navigate("/");
    }
  }, [route, setupRequired]);

  if (setupRequired === null) {
    return null;
  }

  if (setupRequired && route === "/setup/configure") {
    return <ConfigurePage onComplete={() => navigate("/")} />;
  }

  if (setupRequired) {
    return <UnlockPage onUnlocked={() => navigate("/setup/configure")} />;
  }

  return <ReadyPage />;
}
