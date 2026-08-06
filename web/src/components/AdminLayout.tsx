import {
  BarChart3,
  Boxes,
  Gauge,
  HardDrive,
  LogOut,
  PanelLeftClose,
  PanelLeftOpen,
  Settings,
  Share2
} from "lucide-react";
import { ReactNode, useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { getInstanceSummary } from "../api/dashboardApi";
import logoIcon from "../assets/logo-icon.png";
import { LanguageSwitcher } from "./LanguageSwitcher";
import { ThemeToggle } from "./ThemeToggle";

type AdminLayoutProps = {
  children: ReactNode;
  instanceName?: string;
  username?: string | null;
  onLogout: () => void;
};

export function AdminLayout({ children, instanceName, username, onLogout }: AdminLayoutProps) {
  const { t } = useTranslation();
  const [resolvedInstanceName, setResolvedInstanceName] = useState(instanceName);
  const [version, setVersion] = useState("");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem("pontemesh.sidebarCollapsed") === "true");
  const navItems = [
    { to: "/dashboard", label: t("setup.nav.dashboard"), icon: Gauge, enabled: true },
    { to: "/buckets", label: t("setup.nav.buckets"), icon: Boxes, enabled: true },
    { to: "/objects", label: t("setup.nav.objects"), icon: HardDrive, enabled: true },
    { to: "/replicas", label: t("setup.nav.replicas"), icon: Share2, enabled: true },
    { to: "/metrics", label: t("setup.nav.metrics"), icon: BarChart3, enabled: true },
    { to: "/settings", label: t("setup.nav.settings"), icon: Settings, enabled: true }
  ];

  useEffect(() => {
    if (instanceName) {
      setResolvedInstanceName(instanceName);
      return;
    }
    getInstanceSummary()
      .then((summary) => {
        setResolvedInstanceName(summary.name);
        setVersion(summary.version);
      })
      .catch(() => {
        setResolvedInstanceName(undefined);
        setVersion("");
      });
  }, [instanceName]);

  useEffect(() => {
    localStorage.setItem("pontemesh.sidebarCollapsed", String(sidebarCollapsed));
  }, [sidebarCollapsed]);

  return (
    <div className="admin-shell" data-sidebar={sidebarCollapsed ? "collapsed" : "expanded"}>
      <aside className="admin-sidebar" data-testid="app-sidebar">
        <div className="admin-brand">
          <img src={logoIcon} alt="" aria-hidden="true" />
          <span>Ponte Mesh</span>
        </div>
        <nav className="admin-nav" aria-label={t("setup.nav.primary")}>
          {navItems.map((item) => {
            const Icon = item.icon;
            if (!item.enabled) {
              return (
                <span className="admin-nav__item admin-nav__item--disabled" key={item.to} title={item.label}>
                  <Icon size={18} aria-hidden="true" />
                  <span className="admin-nav__label">{item.label}</span>
                </span>
              );
            }
            return (
              <NavLink className="admin-nav__item" to={item.to} key={item.to} title={item.label}>
                <Icon size={18} aria-hidden="true" />
                <span className="admin-nav__label">{item.label}</span>
              </NavLink>
            );
          })}
        </nav>
        <div className="admin-sidebar__footer">
          {version ? <span className="sidebar-version" data-testid="sidebar-version">v{version}</span> : null}
          <button
            className="sidebar-toggle"
            data-testid="sidebar-toggle"
            type="button"
            aria-label={sidebarCollapsed ? t("setup.nav.expandSidebar") : t("setup.nav.collapseSidebar")}
            title={sidebarCollapsed ? t("setup.nav.expandSidebar") : t("setup.nav.collapseSidebar")}
            onClick={() => setSidebarCollapsed((collapsed) => !collapsed)}
          >
            {sidebarCollapsed ? <PanelLeftOpen size={18} aria-hidden="true" /> : <PanelLeftClose size={18} aria-hidden="true" />}
          </button>
        </div>
      </aside>

      <div className="admin-main">
        <header className="admin-topbar">
          <div>
            <span className="admin-topbar__eyebrow">{t("setup.dashboard.instance")}</span>
            <strong>{resolvedInstanceName ?? t("setup.common.loading")}</strong>
          </div>
          <div className="admin-topbar__actions">
            <ThemeToggle />
            <LanguageSwitcher />
            <span className="admin-user">{username ?? t("setup.auth.admin")}</span>
            <button className="admin-logout" type="button" onClick={onLogout}>
              <LogOut size={17} aria-hidden="true" />
              <span>{t("setup.auth.logout")}</span>
            </button>
          </div>
        </header>
        <main className="admin-content">{children}</main>
      </div>
    </div>
  );
}
