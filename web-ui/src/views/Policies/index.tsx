import React, { useState, useCallback } from 'react';
import {
  Shield,
  Plus,
  Trash2,
  Eye,
  FlaskConical,
  X,
  Loader2,
  Pencil,
  CheckSquare,
} from 'lucide-react';
import {
  fetchPolicies as apiFetchPolicies,
  createPolicy as apiCreatePolicy,
  deletePolicy as apiDeletePolicy,
  updatePolicy as apiUpdatePolicy,
  simulatePolicy as apiSimulatePolicy,
  Policy,
} from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoDismiss } from '../../hooks/useAutoDismiss';
import { useAuthStore } from '../../stores/authStore';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';

const STATUS_BADGE: Record<string, string> = {
  active: 'bg-green-500/15 text-green-400 border-green-500/30',
  enforcing: 'bg-green-500/15 text-green-400 border-green-500/30',
  created: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  pending: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  error: 'bg-red-500/15 text-red-400 border-red-500/30',
};

interface SimulationResult {
  policy: string;
  impact: { flows_affected: number; services_impacted: number; risk_level: string; note?: string };
}

const Policies: React.FC = () => {
  usePageTitle('Policies');
  const userRole = useAuthStore((s) => s.role);
  const isAdmin = userRole === 'admin';
  const [policies, setPolicies] = useState<Policy[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useAutoDismiss<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(false);

  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [newNs, setNewNs] = useState('default');
  const [newSpec, setNewSpec] = useState('{\n  "ingress": [],\n  "egress": []\n}');
  const [creating, setCreating] = useState(false);

  const [detailPolicy, setDetailPolicy] = useState<Policy | null>(null);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  const [simulating, setSimulating] = useState(false);
  const [simResult, setSimResult] = useState<SimulationResult | null>(null);

  // Edit state
  const [editPolicy, setEditPolicy] = useState<Policy | null>(null);
  const [editName, setEditName] = useState('');
  const [editNs, setEditNs] = useState('');
  const [editSpec, setEditSpec] = useState('');
  const [editing, setEditing] = useState(false);

  // Bulk operations state
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [bulkDeleting, setBulkDeleting] = useState(false);
  const [bulkDeleteConfirm, setBulkDeleteConfirm] = useState(false);

  const fetchPolicies = useCallback(async () => {
    setError(null);
    try { setPolicies((await apiFetchPolicies()).data.policies); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch policies'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchPolicies, 30000, autoRefreshOn);

  const handleCreate = async () => {
    if (!newName.trim()) return;
    setCreating(true); setError(null);
    let spec: unknown;
    try {
      spec = JSON.parse(newSpec);
    } catch {
      setError('Invalid JSON in policy spec. Please check syntax.');
      setCreating(false);
      return;
    }
    try {
      await apiCreatePolicy({ name: newName.trim(), namespace: newNs.trim() || 'default', spec });
      setCreateOpen(false); setNewName(''); setNewNs('default'); setNewSpec('{\n  "ingress": [],\n  "egress": []\n}');
      setSuccess('Policy created successfully'); fetchPolicies();
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : String(err)); }
    finally { setCreating(false); }
  };

  const handleDelete = async () => {
    if (!deleteId) return;
    setDeleting(true); setError(null);
    try { await apiDeletePolicy(deleteId); setDeleteId(null); setSuccess('Policy deleted'); fetchPolicies(); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Delete failed'); }
    finally { setDeleting(false); }
  };

  const handleUpdate = async () => {
    if (!editPolicy) return;
    setEditing(true); setError(null);
    let spec: unknown;
    try {
      spec = JSON.parse(editSpec);
    } catch {
      setError('Invalid JSON in policy spec. Please check syntax.');
      setEditing(false);
      return;
    }
    try {
      await apiUpdatePolicy(editPolicy.id, { name: editName.trim(), namespace: editNs.trim(), spec });
      setEditPolicy(null); setSuccess('Policy updated successfully'); fetchPolicies();
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : String(err)); }
    finally { setEditing(false); }
  };

  const handleBulkDelete = async () => {
    setBulkDeleting(true); setError(null);
    try {
      await Promise.all([...selectedIds].map(id => apiDeletePolicy(id)));
      const count = selectedIds.size;
      setSelectedIds(new Set()); setBulkDeleteConfirm(false);
      setSuccess(`Deleted ${count} polic${count === 1 ? 'y' : 'ies'}`); fetchPolicies();
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Bulk delete failed'); }
    finally { setBulkDeleting(false); }
  };

  const handleSimulate = async () => {
    setSimulating(true); setError(null);
    let spec: unknown;
    try {
      spec = JSON.parse(newSpec);
    } catch {
      setError('Invalid JSON in policy spec. Please check syntax.');
      setSimulating(false);
      return;
    }
    try {
      const res = await apiSimulatePolicy({ name: newName.trim() || 'sim-test', namespace: newNs.trim() || 'default', spec });
      setSimResult(res.data);
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Simulation failed'); }
    finally { setSimulating(false); }
  };

  const openEdit = (p: Policy) => {
    setEditPolicy(p);
    setEditName(p.name);
    setEditNs(p.namespace);
    setEditSpec(JSON.stringify(p.spec ?? { ingress: [], egress: [] }, null, 2));
  };

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === policies.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(policies.map((p) => p.id)));
    }
  };

  const fmtTs = (ts: string) => { try { const d = new Date(ts); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')} ${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`; } catch { return ts || '-'; } };

  const active = policies.filter((p) => p.status === 'active' || p.status === 'enforcing').length;
  const pending = policies.filter((p) => p.status === 'pending' || p.status === 'created').length;
  const errors = policies.filter((p) => p.status === 'error').length;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20"><Shield className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Policy Management</h1></div>
          <p className="text-sm text-slate-400 mt-1">Manage CiliumNetworkPolicies</p>
        </div>
        <div className="flex items-center gap-2">
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
            autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
          <button onClick={() => setCreateOpen(true)} disabled={!isAdmin} className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed" title={isAdmin ? '' : 'Admin role required'}>
            <Plus className="w-4 h-4" /> Create Policy
          </button>
        </div>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {success && <div className="mb-4 p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-green-400 text-sm">{success}</div>}

      {!isAdmin && (
        <div className="mb-4 p-3 rounded-lg bg-yellow-500/10 border border-yellow-500/30 text-yellow-400 text-sm">
          You have read-only access. Admin role is required for create, edit, and delete operations.
        </div>
      )}

      {/* Bulk action bar */}
      {selectedIds.size > 0 && (
        <div className="mb-4 p-3 rounded-lg bg-blue-500/10 border border-blue-500/30 flex items-center justify-between animate-scale-in">
          <span className="text-sm text-blue-400 flex items-center gap-2">
            <CheckSquare className="w-4 h-4" />
            {selectedIds.size} polic{selectedIds.size === 1 ? 'y' : 'ies'} selected
          </span>
          <div className="flex gap-2">
            <button onClick={() => setSelectedIds(new Set())} className="px-3 py-1.5 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">Clear</button>
            <button onClick={() => setBulkDeleteConfirm(true)} disabled={!isAdmin} className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-red-600 text-white text-sm hover:bg-red-600/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed" title={isAdmin ? '' : 'Admin role required'}>
              <Trash2 className="w-4 h-4" /> Delete Selected
            </button>
          </div>
        </div>
      )}

      {/* Summary */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
          <div className="text-xs text-slate-400">Total</div>
          <div className="text-xl font-bold text-white">{policies.length}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
          <div className="text-xs text-slate-400">Active</div>
          <div className="text-xl font-bold text-green-400">{active}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
          <div className="text-xs text-slate-400">Pending</div>
          <div className="text-xl font-bold text-yellow-400">{pending}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
          <div className="text-xs text-slate-400">Errors</div>
          <div className="text-xl font-bold text-red-400">{errors}</div>
        </div>
      </div>

      {/* Table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-700/50 flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
            <Shield className="w-4 h-4 text-white" />
          </div>
          <h2 className="text-lg font-semibold text-white">Policy Rules</h2>
        </div>
        {loading && <div className="flex justify-center p-3"><Loader2 className="w-5 h-5 animate-spin text-blue-400" /></div>}
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-700/50 bg-slate-900/50">
              <th className="px-4 py-3 w-10">
                <input
                  type="checkbox"
                  checked={policies.length > 0 && selectedIds.size === policies.length}
                  onChange={toggleSelectAll}
                  className="rounded border-slate-600 bg-slate-900/50 text-blue-500 focus:ring-blue-500 focus:ring-offset-0 cursor-pointer"
                />
              </th>
              <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Name</th>
              <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Namespace</th>
              <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Status</th>
              <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Created</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody>
            {policies.length === 0 && !loading ? (
              <tr><td colSpan={6} className="px-4 py-12 text-center text-slate-400">No policies found. Create one to get started.</td></tr>
            ) : policies.map((p) => (
              <tr key={p.id} className="border-b border-slate-700/30 table-row-hover">
                <td className="px-4 py-2.5">
                  <input
                    type="checkbox"
                    checked={selectedIds.has(p.id)}
                    onChange={() => toggleSelect(p.id)}
                    className="rounded border-slate-600 bg-slate-900/50 text-blue-500 focus:ring-blue-500 focus:ring-offset-0 cursor-pointer"
                  />
                </td>
                <td className="px-4 py-2.5 font-medium text-white">{p.name}</td>
                <td className="px-4 py-2.5"><span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{p.namespace}</span></td>
                <td className="px-4 py-2.5">
                  <span className={`px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[p.status] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>{p.status}</span>
                </td>
                <td className="px-4 py-2.5 text-slate-400">{fmtTs(p.created_at)}</td>
                <td className="px-4 py-2.5 text-right">
                  <button onClick={() => setDetailPolicy(p)} className="p-1.5 rounded hover:bg-slate-700/30 text-slate-400 hover:text-white" title="View"><Eye className="w-4 h-4" /></button>
                  <button onClick={() => openEdit(p)} disabled={!isAdmin} className="p-1.5 rounded hover:bg-slate-700/30 text-slate-400 hover:text-blue-400 ml-1 disabled:opacity-50 disabled:cursor-not-allowed" title={isAdmin ? 'Edit' : 'Admin role required'}><Pencil className="w-4 h-4" /></button>
                  <button onClick={() => setDeleteId(p.id)} disabled={!isAdmin} className="p-1.5 rounded hover:bg-slate-700/30 text-slate-400 hover:text-red-400 ml-1 disabled:opacity-50 disabled:cursor-not-allowed" title={isAdmin ? 'Delete' : 'Admin role required'}><Trash2 className="w-4 h-4" /></button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Create dialog */}
      {createOpen && (
        <Modal title="Create Network Policy" onClose={() => setCreateOpen(false)}>
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-white mb-1">Policy Name</label>
                <input value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="my-network-policy" className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
              </div>
              <div>
                <label className="block text-sm font-medium text-white mb-1">Namespace</label>
                <input value={newNs} onChange={(e) => setNewNs(e.target.value)} placeholder="default" className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-white mb-1">Policy Spec (JSON)</label>
              <textarea value={newSpec} onChange={(e) => setNewSpec(e.target.value)} rows={10} className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500" />
            </div>
          </div>
          <div className="flex justify-end gap-2 mt-6 pt-4 border-t border-slate-700/50">
            <button onClick={handleSimulate} disabled={simulating} className="flex items-center gap-2 px-4 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">
              {simulating ? <Loader2 className="w-4 h-4 animate-spin" /> : <FlaskConical className="w-4 h-4" />} Simulate
            </button>
            <button onClick={() => setCreateOpen(false)} className="px-4 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">Cancel</button>
            <button onClick={handleCreate} disabled={creating || !newName.trim()} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 disabled:opacity-50 transition-colors">
              {creating ? <Loader2 className="w-4 h-4 animate-spin" /> : <Plus className="w-4 h-4" />} Create
            </button>
          </div>
        </Modal>
      )}

      {/* Edit dialog */}
      {editPolicy && (
        <Modal title="Edit Network Policy" onClose={() => setEditPolicy(null)}>
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-white mb-1">Policy Name</label>
                <input value={editName} onChange={(e) => setEditName(e.target.value)} placeholder="my-network-policy" className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
              </div>
              <div>
                <label className="block text-sm font-medium text-white mb-1">Namespace</label>
                <input value={editNs} onChange={(e) => setEditNs(e.target.value)} placeholder="default" className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-white mb-1">Policy Spec (JSON)</label>
              <textarea value={editSpec} onChange={(e) => setEditSpec(e.target.value)} rows={10} className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500" />
            </div>
          </div>
          <div className="flex justify-end gap-2 mt-6 pt-4 border-t border-slate-700/50">
            <button onClick={() => setEditPolicy(null)} className="px-4 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">Cancel</button>
            <button onClick={handleUpdate} disabled={editing || !editName.trim()} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 disabled:opacity-50 transition-colors">
              {editing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Pencil className="w-4 h-4" />} Update
            </button>
          </div>
        </Modal>
      )}

      {/* Detail dialog */}
      {detailPolicy && (
        <Modal title="Policy Details" onClose={() => setDetailPolicy(null)}>
          <div className="space-y-3 text-sm">
            <div><span className="text-slate-400">ID:</span> <span className="text-white">{detailPolicy.id}</span></div>
            <div><span className="text-slate-400">Name:</span> <span className="text-white font-medium">{detailPolicy.name}</span></div>
            <div><span className="text-slate-400">Namespace:</span> <span className="text-white">{detailPolicy.namespace}</span></div>
            <div><span className="text-slate-400">Status:</span> <span className={`ml-1 px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[detailPolicy.status] ?? ''}`}>{detailPolicy.status}</span></div>
            <div><span className="text-slate-400">Created:</span> <span className="text-white">{fmtTs(detailPolicy.created_at)}</span></div>
            <div className="pt-3 border-t border-slate-700/50">
              <div className="text-slate-400 mb-2">Policy Data</div>
              <pre className="p-3 rounded-xl bg-slate-950 border border-slate-800 text-xs font-mono text-white overflow-auto max-h-60">
{JSON.stringify(detailPolicy, null, 2)}
              </pre>
            </div>
          </div>
        </Modal>
      )}

      {/* Delete confirm */}
      {deleteId && (
        <Modal title="Delete Policy" onClose={() => setDeleteId(null)}>
          <p className="text-sm text-slate-400 mb-6">Are you sure you want to delete this policy? This action cannot be undone.</p>
          <div className="flex justify-end gap-2">
            <button onClick={() => setDeleteId(null)} className="px-4 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">Cancel</button>
            <button onClick={handleDelete} disabled={deleting} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-red-600 text-red-400-foreground text-sm hover:bg-red-600/90 disabled:opacity-50 transition-colors">
              {deleting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />} Delete
            </button>
          </div>
        </Modal>
      )}

      {/* Bulk delete confirm */}
      {bulkDeleteConfirm && (
        <Modal title="Delete Selected Policies" onClose={() => setBulkDeleteConfirm(false)}>
          <p className="text-sm text-slate-400 mb-6">
            Are you sure you want to delete {selectedIds.size} polic{selectedIds.size === 1 ? 'y' : 'ies'}? This action cannot be undone.
          </p>
          <div className="flex justify-end gap-2">
            <button onClick={() => setBulkDeleteConfirm(false)} className="px-4 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">Cancel</button>
            <button onClick={handleBulkDelete} disabled={bulkDeleting} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-red-600 text-white text-sm hover:bg-red-600/90 disabled:opacity-50 transition-colors">
              {bulkDeleting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />} Delete {selectedIds.size}
            </button>
          </div>
        </Modal>
      )}

      {/* Simulation result */}
      {simResult && (
        <Modal title="Simulation Result" onClose={() => setSimResult(null)}>
          <div className="space-y-4">
            <div className="text-sm text-slate-400">Policy: {simResult.policy}</div>
            <div className="grid grid-cols-2 gap-4">
              <div className="rounded-xl border border-slate-700/50 p-3">
                <div className="text-xs text-slate-400">Flows Affected</div>
                <div className="text-2xl font-bold text-white">{simResult.impact.flows_affected}</div>
              </div>
              <div className="rounded-xl border border-slate-700/50 p-3">
                <div className="text-xs text-slate-400">Services Impacted</div>
                <div className="text-2xl font-bold text-white">{simResult.impact.services_impacted}</div>
              </div>
            </div>
            <div className="text-sm">
              <span className="text-slate-400">Risk Level: </span>
              <span className={`px-2 py-0.5 rounded-full text-xs border ${simResult.impact.risk_level === 'low' ? 'bg-green-500/15 text-green-400 border-green-500/30' : simResult.impact.risk_level === 'high' ? 'bg-red-500/15 text-red-400 border-red-500/30' : 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30'}`}>{simResult.impact.risk_level}</span>
            </div>
            {simResult.impact.note && <div className="p-3 rounded-lg bg-blue-500/10 border border-blue-500/30 text-blue-400 text-sm">{simResult.impact.note}</div>}
          </div>
        </Modal>
      )}
    </div>
  );
};

function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <div className="fixed inset-0 z-50 modal-backdrop animate-fade-in flex items-center justify-center p-4" onClick={onClose} role="dialog" aria-modal="true" aria-label={title}>
      <div className="w-full max-w-lg bg-slate-800/50 border border-slate-700/50 rounded-xl shadow-2xl animate-scale-in" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700/50">
          <h2 className="text-lg font-semibold text-white">{title}</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-white" aria-label="Close"><X className="w-5 h-5" /></button>
        </div>
        <div className="px-6 py-4">{children}</div>
      </div>
    </div>
  );
}

export default Policies;
