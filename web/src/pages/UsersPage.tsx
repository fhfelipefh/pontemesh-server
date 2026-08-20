import { useEffect, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Trash2, UserPlus, Eye, EyeOff } from "lucide-react";
import {
  AdminUserSummary,
  listAdminUsers,
  createAdminUser,
  deleteAdminUser,
} from "../api/usersApi";
import { AuthUser, getCurrentUser } from "../api/authApi";
import { Button } from "../components/Button";
import { ConfirmDialog } from "../components/AdminListControls";
import { CredentialTable } from "../components/settings/CredentialTable";
import { EmptyState } from "../components/settings/EmptyState";
import { IconButton } from "../components/settings/IconButton";
import { SettingsSection } from "../components/settings/SettingsSection";

const isPasswordStrong = (pwd: string) => /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[\W_])[A-Za-z\d\W_]{12,64}$/.test(pwd);

export function UsersPage() {
  const { t, i18n } = useTranslation();
  const [users, setUsers] = useState<AdminUserSummary[]>([]);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [newUsername, setNewUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newRole, setNewRole] = useState("user");
  const [deletingUserId, setDeletingUserId] = useState<string | null>(null);
  const [currentUser, setCurrentUser] = useState<AuthUser | null>(null);
  const [showNewPassword, setShowNewPassword] = useState(false);
  const [showCurrentPassword, setShowCurrentPassword] = useState(false);

  const refreshUsers = useCallback(async () => {
    try {
      const data = await listAdminUsers();
      setUsers(data);
    } catch (loadError) {
      setError(
        loadError instanceof Error
          ? loadError.message
          : t("setup.users.loadFailed")
      );
    }
  }, [t]);

  useEffect(() => {
    void refreshUsers();
    getCurrentUser().then(setCurrentUser).catch(() => {});
  }, [refreshUsers]);

  async function handleCreateUser() {
    setSaving(true);
    setError("");
    try {
      await createAdminUser({
        username: newUsername.trim(),
        password: newPassword,
        currentPassword: currentPassword,
        role: newRole,
      });
      setNewUsername("");
      setNewPassword("");
      setCurrentPassword("");
      await refreshUsers();
    } catch (saveError) {
      setError(
        saveError instanceof Error
          ? saveError.message
          : t("setup.users.createFailed")
      );
    } finally {
      setSaving(false);
    }
  }

  async function handleDeleteUser(id: string) {
    setError("");
    try {
      await deleteAdminUser(id);
      setDeletingUserId(null);
      await refreshUsers();
    } catch (deleteError) {
      setError(
        deleteError instanceof Error
          ? deleteError.message
          : t("setup.users.deleteFailed")
      );
    }
  }

  const formatDate = (isoStr: string) => {
    try {
      return new Intl.DateTimeFormat(i18n.language, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(new Date(isoStr));
    } catch {
      return isoStr;
    }
  };

  const userToDelete = users.find((u) => u.id === deletingUserId);

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.users.title")}</h1>
          <p>{t("setup.users.description")}</p>
        </div>
      </header>

      {error ? <p className="error-message">{error}</p> : null}

      <div className="settings-page__grid">
        <SettingsSection
          className="settings-card--wide"
          title={t("setup.users.listTitle")}
          icon={<UserPlus size={20} />}
        >
          <CredentialTable
            columns={[
              { key: "id", label: "ID" },
              { key: "username", label: t("setup.users.username") },
              { key: "role", label: t("setup.users.role") },
              { key: "createdAt", label: t("setup.users.createdAt") },
              { key: "lastLogin", label: t("setup.users.lastLogin") },
              { key: "actions", label: t("setup.users.actions"), className: "settings-table__col-actions" },
            ]}
          >
            {users.length === 0 ? (
              <tr className="settings-table__empty-row">
                <td colSpan={6}>
                  <EmptyState title={t("setup.users.emptyList")} />
                </td>
              </tr>
            ) : (
              users.map((u) => (
                <tr key={u.id}>
                  <td className="settings-table__id"><code>{u.id.substring(0, 8)}</code></td>
                  <td className="settings-table__name">{u.username}</td>
                  <td>{u.role === "admin" ? t("setup.users.roleAdmin") : t("setup.users.roleUser")}</td>
                  <td>{formatDate(u.createdAt)}</td>
                  <td>{u.lastLoginAt ? formatDate(u.lastLoginAt) : t("setup.common.never")}</td>
                  <td className="settings-table__actions">
                    <IconButton
                      variant="danger"
                      label={t("setup.users.delete")}
                      icon={<Trash2 size={16} aria-hidden="true" />}
                      onClick={() => setDeletingUserId(u.id)}
                      disabled={u.username === currentUser?.username}
                    />
                  </td>
                </tr>
              ))
            )}
          </CredentialTable>
        </SettingsSection>

        <SettingsSection
          title={t("setup.users.createTitle")}
          icon={<UserPlus size={20} />}
        >
          <form
            className="inline-form admin-users-form"
            onSubmit={(event) => {
              event.preventDefault();
              handleCreateUser();
            }}
          >
            <label className="admin-users-field">
              <span>{t("setup.users.newUsername")}</span>
              <input
                value={newUsername}
                onChange={(e) => setNewUsername(e.target.value)}
                placeholder={t("setup.users.newUsername")}
                required
              />
            </label>
            <label className="admin-users-field" style={{ position: "relative" }}>
              <span>{t("setup.users.newPassword")}</span>
              <input
                type={showNewPassword ? "text" : "password"}
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder={t("setup.users.newPassword")}
                style={newPassword && !isPasswordStrong(newPassword) ? { borderColor: "var(--color-danger, red)", paddingRight: "36px" } : { paddingRight: "36px" }}
                required
              />
              <button
                type="button"
                onClick={() => setShowNewPassword(!showNewPassword)}
                style={{ position: "absolute", right: "12px", bottom: "10px", background: "none", border: "none", cursor: "pointer", color: "inherit", padding: 0 }}
                aria-label="Toggle password visibility"
              >
                {showNewPassword ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </label>
            <label className="admin-users-field">
              <span>{t("setup.users.role")}</span>
              <select value={newRole} onChange={(e) => setNewRole(e.target.value)}>
                <option value="user">{t("setup.users.roleUser")}</option>
                <option value="admin">{t("setup.users.roleAdmin")}</option>
              </select>
            </label>
            <label className="admin-users-field" style={{ position: "relative" }}>
              <span>{t("setup.users.confirmCurrentPassword")}</span>
              <input
                type={showCurrentPassword ? "text" : "password"}
                value={currentPassword}
                onChange={(e) => setCurrentPassword(e.target.value)}
                placeholder={t("setup.users.confirmCurrentPassword")}
                style={{ paddingRight: "36px" }}
                required
              />
              <button
                type="button"
                onClick={() => setShowCurrentPassword(!showCurrentPassword)}
                style={{ position: "absolute", right: "12px", bottom: "10px", background: "none", border: "none", cursor: "pointer", color: "inherit", padding: 0 }}
                aria-label="Toggle password visibility"
              >
                {showCurrentPassword ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </label>
            <Button
              type="submit"
              disabled={saving || !newUsername.trim() || !isPasswordStrong(newPassword) || !currentPassword}
              icon={<UserPlus size={17} />}
            >
              {t("setup.users.create")}
            </Button>
          </form>
          {newPassword && !isPasswordStrong(newPassword) ? (
            <p className="admin-users-help">{t("setup.configure.adminPasswordPolicy")}</p>
          ) : null}
        </SettingsSection>
      </div>

      {deletingUserId ? (
        <ConfirmDialog
          title={t("setup.users.confirmDeleteTitle")}
          description={userToDelete?.username ?? ""}
          confirmLabel={t("setup.common.confirm")}
          onConfirm={() => handleDeleteUser(deletingUserId)}
          onCancel={() => setDeletingUserId(null)}
        />
      ) : null}
    </div>
  );
}
