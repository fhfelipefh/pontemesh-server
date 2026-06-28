import {
  BarChart3,
  Boxes,
  Gauge,
  HardDrive,
  LogOut,
  Settings,
  Share2
} from "lucide-react";
import { ReactNode, useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { getInstanceSummary } from "../api/dashboardApi";
import logoIcon from "../assets/logo-icon.png";
import { LanguageSwitcher } from "./LanguageSwitcher";

type AdminLayoutProps = {
  children: ReactNode;
  instanceName?: string;
  username?: string | null;
  onLogout: () => void;
};

export function AdminLayout({ children, instanceName, username, onLogout }: AdminLayoutProps) {
  const { t } = useTranslation();
  const [resolvedInstanceName, setResolvedInstanceName] = useState(instanceName);
  const navItems = [
    { to: "/dashboard", label: t("setup.nav.dashboard"), icon: Gauge, enabled: true },
    { to: "/buckets", label: t("setup.nav.buckets"), icon: Boxes, enabled: true },
    { to: "/objects", label: t("setup.nav.objects"), icon: HardDrive, enabled: false },
    { to: "/replicas", label: t("setup.nav.replicas"), icon: Share2, enabled: false },
    { to: "/metrics", label: t("setup.nav.metrics"), icon: BarChart3, enabled: false },
    { to: "/settings", label: t("setup.nav.settings"), icon: Settings, enabled: false }
  ];

  useEffect(() => {
    if (instanceName) {
      setResolvedInstanceName(instanceName);
      return;
    }
    getInstanceSummary()
      .then((summary) => setResolvedInstanceName(summary.name))
      .catch(() => setResolvedInstanceName(undefined));
  }, [instanceName]);

  return (
    <div className="admin-shell">
      <aside className="admin-sidebar">
        <div className="admin-brand">
          <img src={logoIcon} alt="" aria-hidden="true" />
          <span>Ponte Mesh</span>
        </div>
        <nav className="admin-nav" aria-label={t("setup.nav.primary")}>
          {navItems.map((item) => {
            const Icon = item.icon;
            if (!item.enabled) {
              return (
                <span className="admin-nav__item admin-nav__item--disabled" key={item.to}>
                  <Icon size={18} aria-hidden="true" />
                  {item.label}
                </span>
              );
            }
            return (
              <NavLink className="admin-nav__item" to={item.to} key={item.to}>
                <Icon size={18} aria-hidden="true" />
                {item.label}
              </NavLink>
            );
          })}
        </nav>
      </aside>

      <div className="admin-main">
        <header className="admin-topbar">
          <div>
            <span className="admin-topbar__eyebrow">{t("setup.dashboard.instance")}</span>
            <strong>{resolvedInstanceName ?? t("setup.common.loading")}</strong>
          </div>
          <div className="admin-topbar__actions">
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
