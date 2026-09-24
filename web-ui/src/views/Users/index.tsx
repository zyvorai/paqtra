import { useCallback, useState } from 'react';
import axios from 'axios';
import {
  fetchUsers, createUser, updateUser, deleteUser, apiErrorMessage, AppUser, UserRole,
} from '../../services/api';
import { useAuthStore } from '../../stores/authStore';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

const ROLES: UserRole[] = ['viewer', 'admin'];
const ROLE_HELP: Record<UserRole, string> = {
  admin: 'Full access, including managing users.',
  viewer: 'Read-only: can look at everything, change nothing.',
};

export default function Users() {
  const me = useAuthStore((s) => s.username);
  const [users, setUsers] = useState<AppUser[]>([]);
  const [configAdmin, setConfigAdmin] = useState('');
  const [forbidden, setForbidden] = useState(false);
  const [err, setErr] = useState('');
  const [msg, setMsg] = useState('');
  const [busy, setBusy] = useState<string | null>(null);

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [role, setRole] = useState<UserRole>('viewer');

  const [resetFor, setResetFor] = useState<string | null>(null);
  const [resetPw, setResetPw] = useState('');

  const load = useCallback(async () => {
    try {
      const { data } = await fetchUsers();
      setUsers(data.users ?? []);
      setConfigAdmin(data.config_admin ?? '');
      setForbidden(false);
    } catch (e) {
      if (axios.isAxiosError(e) && e.response?.status === 403) setForbidden(true);
      else setErr(apiErrorMessage(e, 'Failed to load users'));
    }
  }, []);

  // Fetches on mount; no polling, so an admin's half-typed form is never disturbed.
  useAutoRefresh(load, 30000, false);

  const run = async (key: string, action: () => Promise<unknown>, ok: string) => {
    setBusy(key);
    setErr('');
    try {
      await action();
      setMsg(ok);
      await load();
      return true;
    } catch (e) {
      setMsg('');
      setErr(apiErrorMessage(e, 'Request failed'));
      return false;
    } finally {
      setBusy(null);
    }
  };

  const handleCreate = async () => {
    const created = await run('create', () => createUser({ username: username.trim(), password, role }), `Created ${username.trim().toLowerCase()}`);
    if (created) { setUsername(''); setPassword(''); setRole('viewer'); }
  };

  const handleReset = async (u: AppUser) => {
    const done = await run(`reset:${u.username}`, () => updateUser(u.username, { password: resetPw }), `Password reset for ${u.username}; they are signed out everywhere`);
    if (done) { setResetFor(null); setResetPw(''); }
  };

  const handleDelete = async (u: AppUser) => {
    if (!window.confirm(`Delete ${u.username}? They will be signed out immediately.`)) return;
    await run(`del:${u.username}`, () => deleteUser(u.username), `Deleted ${u.username}`);
  };

  if (forbidden) {
    return (
      <Board>
        <Card span={3}>
          <Warning>Admin role required to manage users.</Warning>
        </Card>
      </Board>
    );
  }

  const admins = users.filter((u) => u.role === 'admin' && u.enabled).length;

  return (
    <Board>
      {err ? <Card span={3}><Warning>{err}</Warning></Card> : null}
      {msg ? <Card span={3}><p className="empty-state">{msg}</p></Card> : null}

      <Card span={3}>
        <Eyebrow>ACCESS</Eyebrow>
        <h3>Users and roles</h3>
        <Metrics>
          <Metric value={users.length} label="local users" />
          <Metric value={admins} label="enabled admins" />
        </Metrics>
        <p>
          Changes apply immediately: disabling a user or changing their role takes effect on their next request, and a
          password reset signs them out everywhere. {configAdmin ? <>The account <code>{configAdmin}</code> comes from the server configuration and always works as admin.</> : null}
        </p>
      </Card>

      <Card span={3}>
        <Eyebrow>ADD USER</Eyebrow>
        <Toolbar>
          <label>
            Username
            <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="alice" autoComplete="off" />
          </label>
          <label>
            Password (12+ characters)
            <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="new-password" />
          </label>
          <label>
            Role
            <select value={role} onChange={(e) => setRole(e.target.value as UserRole)}>
              {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
            </select>
          </label>
          <button type="button" className="primary" disabled={busy === 'create' || !username.trim() || !password} onClick={handleCreate}>
            Add user
          </button>
        </Toolbar>
        <p>{ROLE_HELP[role]}</p>
      </Card>

      <Card span={3}>
        <Eyebrow>ACCOUNTS</Eyebrow>
        {users.length === 0 ? <Empty>No local users yet. Sign-in currently uses only the configured admin account.</Empty> : null}
        <div className="list">
          {users.map((u) => {
            const self = u.username === me;
            return (
              <div className="agent wide" key={u.username}>
                <b>{u.username}{self ? ' (you)' : ''}</b>
                <select
                  aria-label={`Role for ${u.username}`}
                  value={u.role}
                  disabled={self || busy !== null}
                  title={self ? 'You cannot change your own role' : ROLE_HELP[u.role]}
                  onChange={(e) => void run(`role:${u.username}`, () => updateUser(u.username, { role: e.target.value as UserRole }), `${u.username} is now ${e.target.value}`)}
                >
                  {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
                </select>
                <small>{u.enabled ? 'enabled' : 'disabled'} · created {new Date(u.created_at).toLocaleDateString()}</small>
                <button
                  type="button"
                  className="btn-refresh"
                  disabled={self || busy !== null}
                  title={self ? 'You cannot disable your own account' : undefined}
                  onClick={() => void run(`en:${u.username}`, () => updateUser(u.username, { enabled: !u.enabled }), `${u.username} ${u.enabled ? 'disabled' : 'enabled'}`)}
                >
                  {u.enabled ? 'Disable' : 'Enable'}
                </button>
                <button type="button" className="btn-refresh" disabled={busy !== null} onClick={() => { setResetFor(resetFor === u.username ? null : u.username); setResetPw(''); }}>
                  Reset password
                </button>
                <button type="button" className="danger" disabled={self || busy !== null} title={self ? 'You cannot delete your own account' : undefined} onClick={() => void handleDelete(u)}>
                  Delete
                </button>
                {resetFor === u.username ? (
                  <Toolbar>
                    <label>
                      New password for {u.username}
                      <input type="password" value={resetPw} onChange={(e) => setResetPw(e.target.value)} autoComplete="new-password" />
                    </label>
                    <button type="button" className="primary" disabled={!resetPw || busy !== null} onClick={() => void handleReset(u)}>
                      Set password
                    </button>
                  </Toolbar>
                ) : null}
              </div>
            );
          })}
        </div>
      </Card>
    </Board>
  );
}
