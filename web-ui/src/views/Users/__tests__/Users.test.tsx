import { render, screen, waitFor, fireEvent, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AxiosError, AxiosResponse } from 'axios';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as api from '../../../services/api';
import { useAuthStore } from '../../../stores/authStore';
import Users from '../index';

vi.mock('../../../services/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../services/api')>();
  return { ...actual, fetchUsers: vi.fn(), createUser: vi.fn(), updateUser: vi.fn(), deleteUser: vi.fn() };
});

const m = vi.mocked(api);
const ok = <T,>(data: T) => Promise.resolve({ data } as never);
const httpError = (status: number, error: string) =>
  Promise.reject(new AxiosError('x', String(status), undefined, undefined, { status, data: { error } } as AxiosResponse));

const user = (username: string, role: 'admin' | 'editor' | 'viewer' = 'viewer', enabled = true) =>
  ({ username, role, enabled, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' });

const renderPage = () => render(<MemoryRouter><Users /></MemoryRouter>);

beforeEach(() => {
  vi.resetAllMocks();
  useAuthStore.setState({ username: 'carol', role: 'admin', isAuthenticated: true });
  m.fetchUsers.mockImplementation(() => ok({ users: [user('bob'), user('carol', 'admin')], total: 2, config_admin: 'admin' }));
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});

describe('Users', () => {
  it('lists accounts and names the configured admin', async () => {
    renderPage();
    expect(await screen.findByText('bob')).toBeInTheDocument();
    expect(screen.getByText('carol (you)')).toBeInTheDocument();
    expect(screen.getByText('admin', { selector: 'code' })).toBeInTheDocument();
  });

  it("disables the signed-in user's own role, disable and delete controls", async () => {
    renderPage();
    await screen.findByText('carol (you)');
    expect(screen.getByLabelText('Role for carol')).toBeDisabled();
    expect(screen.getByLabelText('Role for bob')).toBeEnabled();
    const carolRow = screen.getByText('carol (you)').closest('.agent') as HTMLElement;
    expect(carolRow.querySelector('button.danger')).toBeDisabled();
    const bobRow = screen.getByText('bob').closest('.agent') as HTMLElement;
    expect(bobRow.querySelector('button.danger')).toBeEnabled();
  });

  it('creates a user with a trimmed name and the chosen role, then clears the form', async () => {
    m.createUser.mockImplementation(() => ok({}));
    renderPage();
    await screen.findByText('bob');
    fireEvent.change(screen.getByPlaceholderText('alice'), { target: { value: '  Dave ' } });
    const [pw] = document.querySelectorAll('input[type="password"]');
    fireEvent.change(pw, { target: { value: 'a-long-password-1' } });
    fireEvent.change(screen.getByLabelText('Role'), { target: { value: 'admin' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add user' }));
    await waitFor(() => expect(m.createUser).toHaveBeenCalledWith({ username: 'Dave', password: 'a-long-password-1', role: 'admin' }));
    expect(await screen.findByText('Created dave')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('alice')).toHaveValue('');
  });

  it('shows the server reason and keeps the form when creation fails', async () => {
    m.createUser.mockImplementation(() => httpError(409, 'A user with this name already exists'));
    renderPage();
    await screen.findByText('bob');
    fireEvent.change(screen.getByPlaceholderText('alice'), { target: { value: 'bob' } });
    const [pw] = document.querySelectorAll('input[type="password"]');
    fireEvent.change(pw, { target: { value: 'a-long-password-1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add user' }));
    expect(await screen.findByText('A user with this name already exists')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('alice')).toHaveValue('bob');
  });

  it('offers the editor role and explains what each role can do', async () => {
    renderPage();
    await screen.findByText('bob');
    const form = screen.getByLabelText('Role');
    expect(within(form).getAllByRole('option').map((o) => o.getAttribute('value'))).toEqual(['viewer', 'editor', 'admin']);
    expect(screen.getByText(/Read-only/)).toBeInTheDocument();
    fireEvent.change(form, { target: { value: 'editor' } });
    expect(screen.getByText(/Can work incidents, alerts, SLOs and network policies/)).toBeInTheDocument();
    fireEvent.change(form, { target: { value: 'admin' } });
    expect(screen.getByText(/including users, notification channels/)).toBeInTheDocument();
  });

  it('creates an editor', async () => {
    m.createUser.mockImplementation(() => ok({}));
    renderPage();
    await screen.findByText('bob');
    fireEvent.change(screen.getByPlaceholderText('alice'), { target: { value: 'erin' } });
    const [pw] = document.querySelectorAll('input[type="password"]');
    fireEvent.change(pw, { target: { value: 'a-long-password-1' } });
    fireEvent.change(screen.getByLabelText('Role'), { target: { value: 'editor' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add user' }));
    await waitFor(() => expect(m.createUser).toHaveBeenCalledWith({ username: 'erin', password: 'a-long-password-1', role: 'editor' }));
  });

  it('changes a role and reloads', async () => {
    m.updateUser.mockImplementation(() => ok({}));
    renderPage();
    await screen.findByText('bob');
    fireEvent.change(screen.getByLabelText('Role for bob'), { target: { value: 'admin' } });
    await waitFor(() => expect(m.updateUser).toHaveBeenCalledWith('bob', { role: 'admin' }));
    expect(await screen.findByText('bob is now admin')).toBeInTheDocument();
    expect(m.fetchUsers.mock.calls.length).toBeGreaterThan(1);
  });

  it('disables and re-enables another user', async () => {
    m.updateUser.mockImplementation(() => ok({}));
    m.fetchUsers.mockImplementation(() => ok({ users: [user('bob', 'viewer', false), user('carol', 'admin')], total: 2, config_admin: 'admin' }));
    renderPage();
    await screen.findByText('bob');
    fireEvent.click(screen.getByRole('button', { name: 'Enable' }));
    await waitFor(() => expect(m.updateUser).toHaveBeenCalledWith('bob', { enabled: true }));
  });

  it('resets a password through the inline form', async () => {
    m.updateUser.mockImplementation(() => ok({}));
    renderPage();
    await screen.findByText('bob');
    const bobRow = screen.getByText('bob').closest('.agent') as HTMLElement;
    fireEvent.click(bobRow.querySelector('button.btn-refresh:nth-of-type(2)') as HTMLElement);
    const input = await screen.findByLabelText(/New password for bob/);
    fireEvent.change(input, { target: { value: 'brand-new-password' } });
    fireEvent.click(screen.getByRole('button', { name: 'Set password' }));
    await waitFor(() => expect(m.updateUser).toHaveBeenCalledWith('bob', { password: 'brand-new-password' }));
    expect(await screen.findByText(/Password reset for bob/)).toBeInTheDocument();
  });

  it('deletes only after confirmation', async () => {
    m.deleteUser.mockImplementation(() => ok({}));
    renderPage();
    await screen.findByText('bob');
    const bobRow = screen.getByText('bob').closest('.agent') as HTMLElement;
    const del = bobRow.querySelector('button.danger') as HTMLElement;
    vi.mocked(window.confirm).mockReturnValueOnce(false);
    fireEvent.click(del);
    expect(m.deleteUser).not.toHaveBeenCalled();
    fireEvent.click(del);
    await waitFor(() => expect(m.deleteUser).toHaveBeenCalledWith('bob'));
  });

  it('tells non-admins the page needs the admin role', async () => {
    m.fetchUsers.mockImplementation(() => httpError(403, 'Admin role required'));
    renderPage();
    expect(await screen.findByText('Admin role required to manage users.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Add user' })).not.toBeInTheDocument();
  });
});
