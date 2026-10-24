import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Mail as MailIcon,
  Plus,
  Play,
  CheckCircle,
  AlertCircle,
  Trash2,
  Edit2,
  Server,
  Filter,
  Inbox,
  Folder,
  Layers,
  ShieldCheck,
} from 'lucide-react'
import { api } from '../api/client'
import { useTags } from '../api/useTags'
import { useCorrespondents } from '../api/useCorrespondents'
import { useDocumentTypes } from '../api/useDocumentTypes'
import { confirmModal } from '../store/useModalStore'
import type { MailAccount, MailRule } from '../types'

export function Mail() {
  const qc = useQueryClient()
  const [activeTab, setActiveTab] = useState<'rules' | 'accounts' | 'processed'>('rules')

  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()

  // Queries
  const { data: accountsData } = useQuery({
    queryKey: ['mail_accounts'],
    queryFn: () => api.getMailAccounts(),
  })
  const accounts = accountsData?.results ?? []

  const { data: rulesData } = useQuery({
    queryKey: ['mail_rules'],
    queryFn: () => api.getMailRules(),
  })
  const rules = rulesData?.results ?? []

  const { data: processedData } = useQuery({
    queryKey: ['processed_mail'],
    queryFn: () => api.getProcessedMail(),
  })
  const processed = processedData?.results ?? []

  // Modals state
  const [accountModalOpen, setAccountModalOpen] = useState(false)
  const [editingAccount, setEditingAccount] = useState<MailAccount | null>(null)
  const [accountForm, setAccountForm] = useState<Partial<MailAccount>>({
    name: '',
    imap_server: '',
    imap_port: 993,
    imap_security: 2,
    username: '',
    password: '',
    character_set: 'UTF-8',
    is_token: false,
  })
  const [testResult, setTestResult] = useState<{ success: boolean; detail?: string } | null>(null)
  const [isTesting, setIsTesting] = useState(false)

  const [ruleModalOpen, setRuleModalOpen] = useState(false)
  const [editingRule, setEditingRule] = useState<MailRule | null>(null)
  const [ruleForm, setRuleForm] = useState<Partial<MailRule>>({
    name: '',
    account: 0,
    folder: 'INBOX',
    order: 0,
    enabled: true,
    maximum_age: 30,
    consumption_scope: 1,
    attachment_type: 1,
    action: 1,
    action_parameter: '',
    filter_from: '',
    filter_to: '',
    filter_subject: '',
    filter_body: '',
    filter_attachment_filename: '',
    assign_tags: [],
    assign_correspondent: null,
    assign_document_type: null,
  })

  const [processingRuleId, setProcessingRuleId] = useState<number | null>(null)
  const [processStatus, setProcessStatus] = useState<string | null>(null)

  // Account Mutations
  const createAccountMutation = useMutation({
    mutationFn: (data: Partial<MailAccount>) => api.createMailAccount(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['mail_accounts'] })
      setAccountModalOpen(false)
    },
  })

  const updateAccountMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<MailAccount> }) =>
      api.updateMailAccount(id, data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['mail_accounts'] })
      setAccountModalOpen(false)
    },
  })

  const deleteAccountMutation = useMutation({
    mutationFn: (id: number) => api.deleteMailAccount(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['mail_accounts'] }),
  })

  // Rule Mutations
  const createRuleMutation = useMutation({
    mutationFn: (data: Partial<MailRule>) => api.createMailRule(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['mail_rules'] })
      setRuleModalOpen(false)
    },
  })

  const updateRuleMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<MailRule> }) =>
      api.updateMailRule(id, data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['mail_rules'] })
      setRuleModalOpen(false)
    },
  })

  const deleteRuleMutation = useMutation({
    mutationFn: (id: number) => api.deleteMailRule(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['mail_rules'] }),
  })

  const handleTestAccount = async () => {
    setIsTesting(true)
    setTestResult(null)
    try {
      const res = await api.testMailAccount({
        imap_server: accountForm.imap_server || '',
        imap_port: Number(accountForm.imap_port) || 993,
        imap_security: Number(accountForm.imap_security) || 2,
        username: accountForm.username || '',
        password: accountForm.password || '',
      })
      setTestResult(res)
    } catch (err: any) {
      setTestResult({ success: false, detail: err.message || 'Connection test failed' })
    } finally {
      setIsTesting(false)
    }
  }

  const handleProcessRule = async (ruleId: number) => {
    setProcessingRuleId(ruleId)
    setProcessStatus(null)
    try {
      const res = await api.processMailRule(ruleId)
      setProcessStatus(
        `Success: Processed ${res.messages_processed ?? 0} messages, ingested ${
          res.attachments?.length ?? 0
        } attachments.`
      )
      qc.invalidateQueries({ queryKey: ['processed_mail'] })
      qc.invalidateQueries({ queryKey: ['documents'] })
    } catch (err: any) {
      setProcessStatus(`Error running mail rule: ${err.message || 'Worker processing failed'}`)
    } finally {
      setProcessingRuleId(null)
    }
  }

  const openAccountModal = (acc?: MailAccount) => {
    if (acc) {
      setEditingAccount(acc)
      setAccountForm({
        name: acc.name,
        imap_server: acc.imap_server,
        imap_port: acc.imap_port ?? 993,
        imap_security: acc.imap_security ?? 2,
        username: acc.username,
        password: '',
        character_set: acc.character_set ?? 'UTF-8',
        is_token: acc.is_token ?? false,
      })
    } else {
      setEditingAccount(null)
      setAccountForm({
        name: '',
        imap_server: '',
        imap_port: 993,
        imap_security: 2,
        username: '',
        password: '',
        character_set: 'UTF-8',
        is_token: false,
      })
    }
    setTestResult(null)
    setAccountModalOpen(true)
  }

  const openRuleModal = (rule?: MailRule) => {
    if (rule) {
      setEditingRule(rule)
      setRuleForm({
        name: rule.name,
        account: rule.account,
        folder: rule.folder || 'INBOX',
        order: rule.order ?? 0,
        enabled: rule.enabled ?? true,
        maximum_age: rule.maximum_age ?? 30,
        consumption_scope: rule.consumption_scope ?? 1,
        attachment_type: rule.attachment_type ?? 1,
        action: rule.action ?? 1,
        action_parameter: rule.action_parameter || '',
        filter_from: rule.filter_from || '',
        filter_to: rule.filter_to || '',
        filter_subject: rule.filter_subject || '',
        filter_body: rule.filter_body || '',
        filter_attachment_filename: rule.filter_attachment_filename || '',
        assign_tags: rule.assign_tags || [],
        assign_correspondent: rule.assign_correspondent ?? null,
        assign_document_type: rule.assign_document_type ?? null,
      })
    } else {
      setEditingRule(null)
      setRuleForm({
        name: '',
        account: accounts[0]?.id || 0,
        folder: 'INBOX',
        order: (rules.length + 1) * 10,
        enabled: true,
        maximum_age: 30,
        consumption_scope: 1,
        attachment_type: 1,
        action: 1,
        action_parameter: '',
        filter_from: '',
        filter_to: '',
        filter_subject: '',
        filter_body: '',
        filter_attachment_filename: '',
        assign_tags: [],
        assign_correspondent: null,
        assign_document_type: null,
      })
    }
    setRuleModalOpen(true)
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '16px' }}>
        <div>
          <h1 style={{ fontSize: '1.75rem', fontWeight: 800, letterSpacing: '-0.02em', display: 'flex', alignItems: 'center', gap: '12px' }}>
            <MailIcon size={28} style={{ color: 'var(--accent-primary)' }} />
            Mail Fetcher & Automation
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginTop: '4px' }}>
            Configure IMAP accounts and criteria rules to automatically fetch, parse, and ingest documents from email.
          </p>
        </div>

        <div style={{ display: 'flex', gap: '10px' }}>
          {activeTab === 'rules' && (
            <button
              className="btn btn-primary"
              onClick={() => openRuleModal()}
              disabled={accounts.length === 0}
              style={{ gap: '8px' }}
            >
              <Plus size={16} /> Add Mail Rule
            </button>
          )}
          {activeTab === 'accounts' && (
            <button className="btn btn-primary" onClick={() => openAccountModal()} style={{ gap: '8px' }}>
              <Plus size={16} /> Add Mail Account
            </button>
          )}
        </div>
      </div>

      {processStatus && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 'var(--radius-md)',
            background: processStatus.startsWith('Success') ? 'rgba(52, 211, 153, 0.12)' : 'rgba(239, 68, 68, 0.12)',
            border: `1px solid ${processStatus.startsWith('Success') ? 'rgba(52, 211, 153, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
            color: processStatus.startsWith('Success') ? '#34d399' : '#f87171',
            fontSize: '0.88rem',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <span>{processStatus}</span>
          <button
            onClick={() => setProcessStatus(null)}
            style={{ background: 'transparent', border: 'none', color: 'inherit', cursor: 'pointer', fontSize: '1rem' }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Tab Switcher */}
      <div
        style={{
          display: 'flex',
          gap: '8px',
          borderBottom: '1px solid var(--border-subtle)',
          paddingBottom: '2px',
        }}
      >
        <button
          onClick={() => setActiveTab('rules')}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            padding: '10px 18px',
            background: 'none',
            border: 'none',
            borderBottom: activeTab === 'rules' ? '2px solid var(--accent-primary)' : '2px solid transparent',
            color: activeTab === 'rules' ? 'var(--text-primary)' : 'var(--text-muted)',
            fontWeight: activeTab === 'rules' ? 600 : 500,
            fontSize: '0.9rem',
            cursor: 'pointer',
          }}
        >
          <Filter size={16} /> Mail Rules ({rules.length})
        </button>
        <button
          onClick={() => setActiveTab('accounts')}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            padding: '10px 18px',
            background: 'none',
            border: 'none',
            borderBottom: activeTab === 'accounts' ? '2px solid var(--accent-primary)' : '2px solid transparent',
            color: activeTab === 'accounts' ? 'var(--text-primary)' : 'var(--text-muted)',
            fontWeight: activeTab === 'accounts' ? 600 : 500,
            fontSize: '0.9rem',
            cursor: 'pointer',
          }}
        >
          <Server size={16} /> Mail Accounts ({accounts.length})
        </button>
        <button
          onClick={() => setActiveTab('processed')}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            padding: '10px 18px',
            background: 'none',
            border: 'none',
            borderBottom: activeTab === 'processed' ? '2px solid var(--accent-primary)' : '2px solid transparent',
            color: activeTab === 'processed' ? 'var(--text-primary)' : 'var(--text-muted)',
            fontWeight: activeTab === 'processed' ? 600 : 500,
            fontSize: '0.9rem',
            cursor: 'pointer',
          }}
        >
          <Inbox size={16} /> Processed History ({processed.length})
        </button>
      </div>

      {/* RULES TAB */}
      {activeTab === 'rules' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {accounts.length === 0 && (
            <div
              style={{
                padding: '24px',
                borderRadius: 'var(--radius-md)',
                backgroundColor: 'rgba(234, 179, 8, 0.08)',
                border: '1px solid rgba(234, 179, 8, 0.25)',
                color: '#facc15',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                <AlertCircle size={24} />
                <div>
                  <div style={{ fontWeight: 600 }}>No Mail Accounts Configured</div>
                  <div style={{ fontSize: '0.85rem', opacity: 0.9 }}>
                    You must configure at least one IMAP mail account before creating mail rules.
                  </div>
                </div>
              </div>
              <button
                className="btn btn-primary"
                onClick={() => {
                  setActiveTab('accounts')
                  openAccountModal()
                }}
              >
                Setup Account
              </button>
            </div>
          )}

          {rules.length === 0 ? (
            <div
              style={{
                padding: '48px 24px',
                textAlign: 'center',
                backgroundColor: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
                border: '1px dashed var(--border-subtle)',
              }}
            >
              <MailIcon size={40} style={{ color: 'var(--text-muted)', marginBottom: '12px' }} />
              <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>No mail rules configured</h3>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.88rem', margin: '8px auto 20px', maxWidth: '420px' }}>
                Mail rules scan specific IMAP folders (like INBOX), extract PDF or document attachments, and automatically assign tags, correspondents, and document types.
              </p>
              {accounts.length > 0 && (
                <button className="btn btn-primary" onClick={() => openRuleModal()}>
                  <Plus size={16} /> Create First Mail Rule
                </button>
              )}
            </div>
          ) : (
            <div
              style={{
                backgroundColor: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
                border: '1px solid var(--border-subtle)',
                overflow: 'hidden',
              }}
            >
              <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.88rem' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-subtle)', color: 'var(--text-secondary)' }}>
                    <th style={{ padding: '14px 18px', width: '70px' }}>Order</th>
                    <th style={{ padding: '14px 18px' }}>Rule Name</th>
                    <th style={{ padding: '14px 18px' }}>Account & Folder</th>
                    <th style={{ padding: '14px 18px' }}>Filter Criteria</th>
                    <th style={{ padding: '14px 18px' }}>Metadata Assignment</th>
                    <th style={{ padding: '14px 18px', textAlign: 'center', width: '90px' }}>Status</th>
                    <th style={{ padding: '14px 18px', textAlign: 'right' }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {rules.map((rule) => {
                    const acc = accounts.find((a) => a.id === rule.account)
                    const corr = correspondents.find((c) => c.id === rule.assign_correspondent)
                    const dt = documentTypes.find((d) => d.id === rule.assign_document_type)

                    return (
                      <tr
                        key={rule.id}
                        style={{
                          borderBottom: '1px solid var(--border-subtle)',
                          opacity: rule.enabled ? 1 : 0.6,
                        }}
                      >
                        <td style={{ padding: '14px 18px', fontWeight: 600, color: 'var(--text-muted)' }}>
                          #{rule.order}
                        </td>
                        <td style={{ padding: '14px 18px' }}>
                          <div style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{rule.name}</div>
                          <div style={{ fontSize: '0.78rem', color: 'var(--text-muted)' }}>
                            Scope: {rule.consumption_scope === 2 ? 'Everything (Msg+Attachments)' : 'Attachments Only'}
                          </div>
                        </td>
                        <td style={{ padding: '14px 18px' }}>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                            <Server size={14} style={{ color: 'var(--accent-primary)' }} />
                            <span style={{ fontWeight: 500 }}>{acc?.name || `Account #${rule.account}`}</span>
                          </div>
                          <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: '4px', marginTop: '2px' }}>
                            <Folder size={12} /> {rule.folder || 'INBOX'}
                          </div>
                        </td>
                        <td style={{ padding: '14px 18px' }}>
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '3px', fontSize: '0.8rem' }}>
                            {rule.filter_from && (
                              <div>
                                <span style={{ color: 'var(--text-muted)' }}>From: </span>
                                <code>{rule.filter_from}</code>
                              </div>
                            )}
                            {rule.filter_subject && (
                              <div>
                                <span style={{ color: 'var(--text-muted)' }}>Subject: </span>
                                <code>{rule.filter_subject}</code>
                              </div>
                            )}
                            {rule.filter_attachment_filename && (
                              <div>
                                <span style={{ color: 'var(--text-muted)' }}>Attachment: </span>
                                <code>{rule.filter_attachment_filename}</code>
                              </div>
                            )}
                            {!rule.filter_from && !rule.filter_subject && !rule.filter_attachment_filename && (
                              <span style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>All messages in folder</span>
                            )}
                          </div>
                        </td>
                        <td style={{ padding: '14px 18px' }}>
                          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', fontSize: '0.8rem' }}>
                            {corr && (
                              <div>
                                <span style={{ color: 'var(--text-muted)' }}>Correspondent: </span>
                                <span style={{ fontWeight: 600 }}>{corr.name}</span>
                              </div>
                            )}
                            {dt && (
                              <div>
                                <span style={{ color: 'var(--text-muted)' }}>Type: </span>
                                <span style={{ fontWeight: 600 }}>{dt.name}</span>
                              </div>
                            )}
                            {rule.assign_tags && rule.assign_tags.length > 0 && (
                              <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap', marginTop: '2px' }}>
                                {rule.assign_tags.map((tid: number) => {
                                  const t = tags.find((item) => item.id === tid)
                                  return (
                                    <span
                                      key={tid}
                                      style={{
                                        fontSize: '0.72rem',
                                        padding: '1px 6px',
                                        borderRadius: 'var(--radius-sm)',
                                        backgroundColor: t?.color ? `${t.color}22` : 'rgba(99,102,241,0.15)',
                                        color: t?.color || 'var(--accent-primary)',
                                        border: `1px solid ${t?.color || 'rgba(99,102,241,0.3)'}`,
                                      }}
                                    >
                                      {t?.name || `Tag #${tid}`}
                                    </span>
                                  )
                                })}
                              </div>
                            )}
                          </div>
                        </td>
                        <td style={{ padding: '14px 18px', textAlign: 'center' }}>
                          <span
                            style={{
                              fontSize: '0.75rem',
                              padding: '2px 8px',
                              borderRadius: 'var(--radius-full)',
                              backgroundColor: rule.enabled ? 'rgba(52, 211, 153, 0.15)' : 'rgba(148, 163, 184, 0.15)',
                              color: rule.enabled ? '#34d399' : '#94a3b8',
                              fontWeight: 600,
                            }}
                          >
                            {rule.enabled ? 'Active' : 'Disabled'}
                          </span>
                        </td>
                        <td style={{ padding: '14px 18px', textAlign: 'right' }}>
                          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
                            <button
                              className="btn btn-secondary"
                              onClick={() => handleProcessRule(rule.id)}
                              disabled={processingRuleId === rule.id}
                              title="Process Now (Poll IMAP & Ingest)"
                              style={{ padding: '6px 10px', fontSize: '0.8rem', gap: '5px' }}
                            >
                              <Play
                                size={13}
                                style={{
                                  animation: processingRuleId === rule.id ? 'spin 1s linear infinite' : 'none',
                                }}
                              />
                              {processingRuleId === rule.id ? 'Fetching...' : 'Run'}
                            </button>
                            <button
                              className="btn btn-secondary"
                              onClick={() => openRuleModal(rule)}
                              style={{ padding: '6px 10px' }}
                              title="Edit Rule"
                            >
                              <Edit2 size={13} />
                            </button>
                            <button
                              className="btn btn-danger"
                              onClick={async () => {
                                const ok = await confirmModal({
                                  title: 'Delete Mail Rule',
                                  message: `Are you sure you want to delete mail rule "${rule.name}"?`,
                                  confirmText: 'Delete Rule',
                                  type: 'danger',
                                })
                                if (ok) {
                                  deleteRuleMutation.mutate(rule.id)
                                }
                              }}
                              style={{ padding: '6px 10px' }}
                              title="Delete Rule"
                            >
                              <Trash2 size={13} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* ACCOUNTS TAB */}
      {activeTab === 'accounts' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {accounts.length === 0 ? (
            <div
              style={{
                padding: '48px 24px',
                textAlign: 'center',
                backgroundColor: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
                border: '1px dashed var(--border-subtle)',
              }}
            >
              <Server size={40} style={{ color: 'var(--text-muted)', marginBottom: '12px' }} />
              <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>No mail accounts configured</h3>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.88rem', margin: '8px auto 20px', maxWidth: '420px' }}>
                Add your IMAP email account details to allow FileDeck to monitor your inbox and download documents.
              </p>
              <button className="btn btn-primary" onClick={() => openAccountModal()}>
                <Plus size={16} /> Add IMAP Account
              </button>
            </div>
          ) : (
            <div
              style={{
                backgroundColor: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
                border: '1px solid var(--border-subtle)',
                overflow: 'hidden',
              }}
            >
              <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.88rem' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-subtle)', color: 'var(--text-secondary)' }}>
                    <th style={{ padding: '14px 18px' }}>Account Name</th>
                    <th style={{ padding: '14px 18px' }}>IMAP Host</th>
                    <th style={{ padding: '14px 18px' }}>Port & Security</th>
                    <th style={{ padding: '14px 18px' }}>Username</th>
                    <th style={{ padding: '14px 18px' }}>Charset</th>
                    <th style={{ padding: '14px 18px', textAlign: 'right' }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {accounts.map((acc) => (
                    <tr key={acc.id} style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                      <td style={{ padding: '14px 18px', fontWeight: 600, color: 'var(--text-primary)' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                          <Server size={16} style={{ color: 'var(--accent-primary)' }} />
                          {acc.name}
                        </div>
                      </td>
                      <td style={{ padding: '14px 18px' }}>
                        <code>{acc.imap_server}</code>
                      </td>
                      <td style={{ padding: '14px 18px' }}>
                        <span style={{ fontWeight: 500 }}>{acc.imap_port ?? 993}</span>
                        <span style={{ color: 'var(--text-muted)', marginLeft: '6px', fontSize: '0.8rem' }}>
                          ({acc.imap_security === 2 ? 'SSL/TLS' : acc.imap_security === 3 ? 'STARTTLS' : 'None'})
                        </span>
                      </td>
                      <td style={{ padding: '14px 18px' }}>{acc.username}</td>
                      <td style={{ padding: '14px 18px', color: 'var(--text-muted)' }}>
                        {acc.character_set || 'UTF-8'}
                      </td>
                      <td style={{ padding: '14px 18px', textAlign: 'right' }}>
                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
                          <button
                            className="btn btn-secondary"
                            onClick={() => openAccountModal(acc)}
                            style={{ padding: '6px 12px', fontSize: '0.8rem', gap: '5px' }}
                          >
                            <Edit2 size={13} /> Edit
                          </button>
                          <button
                            className="btn btn-danger"
                            onClick={async () => {
                              const ok = await confirmModal({
                                title: 'Delete Mail Account',
                                message: `Are you sure you want to delete mail account "${acc.name}"?`,
                                confirmText: 'Delete Account',
                                type: 'danger',
                              })
                              if (ok) {
                                deleteAccountMutation.mutate(acc.id)
                              }
                            }}
                            style={{ padding: '6px 10px' }}
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* PROCESSED MAIL LOG TAB */}
      {activeTab === 'processed' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {processed.length === 0 ? (
            <div
              style={{
                padding: '48px 24px',
                textAlign: 'center',
                backgroundColor: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
                border: '1px dashed var(--border-subtle)',
              }}
            >
              <Inbox size={40} style={{ color: 'var(--text-muted)', marginBottom: '12px' }} />
              <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>No processed mail history</h3>
              <p style={{ color: 'var(--text-secondary)', fontSize: '0.88rem', margin: '8px auto 0', maxWidth: '420px' }}>
                Messages fetched and processed by mail rules will appear here with ingestion status and attachments.
              </p>
            </div>
          ) : (
            <div
              style={{
                backgroundColor: 'var(--bg-surface)',
                borderRadius: 'var(--radius-lg)',
                border: '1px solid var(--border-subtle)',
                overflow: 'hidden',
              }}
            >
              <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.88rem' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-subtle)', color: 'var(--text-secondary)' }}>
                    <th style={{ padding: '14px 18px' }}>Message ID / UID</th>
                    <th style={{ padding: '14px 18px' }}>Rule ID</th>
                    <th style={{ padding: '14px 18px' }}>Subject</th>
                    <th style={{ padding: '14px 18px' }}>Processed Date</th>
                    <th style={{ padding: '14px 18px', textAlign: 'center' }}>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {processed.map((item, idx) => (
                    <tr key={item.id ?? idx} style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                      <td style={{ padding: '14px 18px' }}>
                        <code style={{ fontSize: '0.78rem' }}>{item.message_id || `UID ${item.uid}`}</code>
                      </td>
                      <td style={{ padding: '14px 18px', color: 'var(--text-muted)' }}>
                        Rule #{item.rule_id}
                      </td>
                      <td style={{ padding: '14px 18px', fontWeight: 500, color: 'var(--text-primary)' }}>
                        {item.subject || '(No subject)'}
                      </td>
                      <td style={{ padding: '14px 18px', color: 'var(--text-secondary)', fontSize: '0.8rem' }}>
                        {item.processed_at ? new Date(item.processed_at).toLocaleString() : 'Just now'}
                      </td>
                      <td style={{ padding: '14px 18px', textAlign: 'center' }}>
                        <span
                          style={{
                            fontSize: '0.75rem',
                            padding: '2px 8px',
                            borderRadius: 'var(--radius-full)',
                            backgroundColor:
                              item.status === 'ERROR'
                                ? 'rgba(239, 68, 68, 0.15)'
                                : 'rgba(52, 211, 153, 0.15)',
                            color: item.status === 'ERROR' ? '#f87171' : '#34d399',
                            fontWeight: 600,
                          }}
                        >
                          {item.status || 'INGESTED'}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* ACCOUNT MODAL */}
      {accountModalOpen && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.7)',
            backdropFilter: 'blur(4px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 1000,
            padding: '20px',
          }}
        >
          <div
            style={{
              backgroundColor: 'var(--bg-surface)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-lg)',
              maxWidth: '560px',
              width: '100%',
              padding: '28px',
              display: 'flex',
              flexDirection: 'column',
              gap: '20px',
              boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <h2 style={{ fontSize: '1.25rem', fontWeight: 700 }}>
                {editingAccount ? 'Edit Mail Account' : 'New Mail Account'}
              </h2>
              <button
                onClick={() => setAccountModalOpen(false)}
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '1.2rem' }}
              >
                ✕
              </button>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                  Account Name
                </label>
                <input
                  type="text"
                  className="input"
                  placeholder="e.g. Work Invoices Inbox"
                  value={accountForm.name || ''}
                  onChange={(e) => setAccountForm({ ...accountForm, name: e.target.value })}
                  style={{ width: '100%' }}
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    IMAP Server
                  </label>
                  <input
                    type="text"
                    className="input"
                    placeholder="e.g. imap.example.com"
                    value={accountForm.imap_server || ''}
                    onChange={(e) => setAccountForm({ ...accountForm, imap_server: e.target.value })}
                    style={{ width: '100%' }}
                  />
                </div>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Port
                  </label>
                  <input
                    type="number"
                    className="input"
                    value={accountForm.imap_port ?? 993}
                    onChange={(e) => setAccountForm({ ...accountForm, imap_port: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  />
                </div>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Security
                  </label>
                  <select
                    className="input"
                    value={accountForm.imap_security ?? 2}
                    onChange={(e) => setAccountForm({ ...accountForm, imap_security: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  >
                    <option value={1}>None</option>
                    <option value={2}>SSL / TLS (Default)</option>
                    <option value={3}>STARTTLS</option>
                  </select>
                </div>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Character Set
                  </label>
                  <input
                    type="text"
                    className="input"
                    value={accountForm.character_set || 'UTF-8'}
                    onChange={(e) => setAccountForm({ ...accountForm, character_set: e.target.value })}
                    style={{ width: '100%' }}
                  />
                </div>
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                  Username (Email Address)
                </label>
                <input
                  type="text"
                  className="input"
                  placeholder="user@example.com"
                  value={accountForm.username || ''}
                  onChange={(e) => setAccountForm({ ...accountForm, username: e.target.value })}
                  style={{ width: '100%' }}
                />
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                  Password / App Token {editingAccount && <span style={{ color: 'var(--text-muted)' }}>(Leave blank to keep unchanged)</span>}
                </label>
                <input
                  type="password"
                  className="input"
                  placeholder="••••••••••••"
                  value={accountForm.password || ''}
                  onChange={(e) => setAccountForm({ ...accountForm, password: e.target.value })}
                  style={{ width: '100%' }}
                />
              </div>

              {testResult && (
                <div
                  style={{
                    padding: '10px 14px',
                    borderRadius: 'var(--radius-md)',
                    backgroundColor: testResult.success ? 'rgba(52, 211, 153, 0.12)' : 'rgba(239, 68, 68, 0.12)',
                    border: `1px solid ${testResult.success ? 'rgba(52, 211, 153, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
                    color: testResult.success ? '#34d399' : '#f87171',
                    fontSize: '0.82rem',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '8px',
                  }}
                >
                  {testResult.success ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
                  <span>{testResult.success ? 'IMAP connection successful!' : testResult.detail || 'Connection failed'}</span>
                </div>
              )}
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '10px' }}>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={handleTestAccount}
                disabled={isTesting || !accountForm.imap_server || !accountForm.username}
                style={{ gap: '6px' }}
              >
                <ShieldCheck size={16} />
                {isTesting ? 'Testing...' : 'Test Connection'}
              </button>

              <div style={{ display: 'flex', gap: '10px' }}>
                <button type="button" className="btn btn-secondary" onClick={() => setAccountModalOpen(false)}>
                  Cancel
                </button>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => {
                    if (editingAccount) {
                      updateAccountMutation.mutate({ id: editingAccount.id, data: accountForm })
                    } else {
                      createAccountMutation.mutate(accountForm)
                    }
                  }}
                  disabled={!accountForm.name || !accountForm.imap_server || !accountForm.username}
                >
                  {editingAccount ? 'Save Changes' : 'Create Account'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* RULE MODAL */}
      {ruleModalOpen && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0, 0, 0, 0.7)',
            backdropFilter: 'blur(4px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 1000,
            padding: '20px',
          }}
        >
          <div
            style={{
              backgroundColor: 'var(--bg-surface)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-lg)',
              maxWidth: '720px',
              width: '100%',
              maxHeight: '90vh',
              overflowY: 'auto',
              padding: '28px',
              display: 'flex',
              flexDirection: 'column',
              gap: '20px',
              boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <h2 style={{ fontSize: '1.25rem', fontWeight: 700 }}>
                  {editingRule ? 'Edit Mail Rule' : 'New Mail Ingestion Rule'}
                </h2>
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.82rem' }}>
                  Filter messages, process attachments, and automatically assign taxonomy
                </div>
              </div>
              <button
                onClick={() => setRuleModalOpen(false)}
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '1.2rem' }}
              >
                ✕
              </button>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '18px' }}>
              {/* General Settings */}
              <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Rule Name
                  </label>
                  <input
                    type="text"
                    className="input"
                    placeholder="e.g. Invoices from Suppliers"
                    value={ruleForm.name || ''}
                    onChange={(e) => setRuleForm({ ...ruleForm, name: e.target.value })}
                    style={{ width: '100%' }}
                  />
                </div>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Account
                  </label>
                  <select
                    className="input"
                    value={ruleForm.account ?? accounts[0]?.id}
                    onChange={(e) => setRuleForm({ ...ruleForm, account: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  >
                    {accounts.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Order
                  </label>
                  <input
                    type="number"
                    className="input"
                    value={ruleForm.order ?? 0}
                    onChange={(e) => setRuleForm({ ...ruleForm, order: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  />
                </div>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Folder
                  </label>
                  <input
                    type="text"
                    className="input"
                    value={ruleForm.folder || 'INBOX'}
                    onChange={(e) => setRuleForm({ ...ruleForm, folder: e.target.value })}
                    style={{ width: '100%' }}
                  />
                </div>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Maximum Age (Days)
                  </label>
                  <input
                    type="number"
                    className="input"
                    value={ruleForm.maximum_age ?? 30}
                    onChange={(e) => setRuleForm({ ...ruleForm, maximum_age: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  />
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '10px', marginTop: '22px' }}>
                  <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer', fontSize: '0.88rem' }}>
                    <input
                      type="checkbox"
                      checked={ruleForm.enabled ?? true}
                      onChange={(e) => setRuleForm({ ...ruleForm, enabled: e.target.checked })}
                    />
                    <span style={{ fontWeight: 600 }}>Enabled</span>
                  </label>
                </div>
              </div>

              {/* Filter Criteria */}
              <div
                style={{
                  padding: '16px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: 'rgba(255, 255, 255, 0.02)',
                  border: '1px solid var(--border-subtle)',
                }}
              >
                <div style={{ fontWeight: 700, fontSize: '0.9rem', marginBottom: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <Filter size={16} style={{ color: 'var(--accent-primary)' }} />
                  Filter Criteria
                </div>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', marginBottom: '12px' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                      Filter From (Sender)
                    </label>
                    <input
                      type="text"
                      className="input"
                      placeholder="e.g. billing@, @amazon.com"
                      value={ruleForm.filter_from || ''}
                      onChange={(e) => setRuleForm({ ...ruleForm, filter_from: e.target.value })}
                      style={{ width: '100%', fontSize: '0.82rem' }}
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                      Filter Subject
                    </label>
                    <input
                      type="text"
                      className="input"
                      placeholder="e.g. Invoice, Receipt, Statement"
                      value={ruleForm.filter_subject || ''}
                      onChange={(e) => setRuleForm({ ...ruleForm, filter_subject: e.target.value })}
                      style={{ width: '100%', fontSize: '0.82rem' }}
                    />
                  </div>
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                      Filter Attachment Filename
                    </label>
                    <input
                      type="text"
                      className="input"
                      placeholder="e.g. *.pdf, invoice*.pdf"
                      value={ruleForm.filter_attachment_filename || ''}
                      onChange={(e) => setRuleForm({ ...ruleForm, filter_attachment_filename: e.target.value })}
                      style={{ width: '100%', fontSize: '0.82rem' }}
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                      Filter Body (Content)
                    </label>
                    <input
                      type="text"
                      className="input"
                      placeholder="e.g. total due"
                      value={ruleForm.filter_body || ''}
                      onChange={(e) => setRuleForm({ ...ruleForm, filter_body: e.target.value })}
                      style={{ width: '100%', fontSize: '0.82rem' }}
                    />
                  </div>
                </div>
              </div>

              {/* Consumption & Action */}
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Consumption Scope
                  </label>
                  <select
                    className="input"
                    value={ruleForm.consumption_scope ?? 1}
                    onChange={(e) => setRuleForm({ ...ruleForm, consumption_scope: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  >
                    <option value={1}>Only process attachments</option>
                    <option value={2}>Process message body and attachments</option>
                  </select>
                </div>

                <div>
                  <label style={{ display: 'block', fontSize: '0.82rem', fontWeight: 600, marginBottom: '6px' }}>
                    Post-Processing Action
                  </label>
                  <select
                    className="input"
                    value={ruleForm.action ?? 1}
                    onChange={(e) => setRuleForm({ ...ruleForm, action: Number(e.target.value) })}
                    style={{ width: '100%' }}
                  >
                    <option value={1}>Mark as read</option>
                    <option value={2}>Flag / Star message</option>
                    <option value={3}>Move to folder</option>
                    <option value={4}>Delete message</option>
                  </select>
                </div>
              </div>

              {/* Metadata Assignment */}
              <div
                style={{
                  padding: '16px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: 'var(--accent-soft)',
                  border: '1px solid var(--accent-border)',
                }}
              >
                <div style={{ fontWeight: 700, fontSize: '0.9rem', marginBottom: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <Layers size={16} style={{ color: 'var(--accent-primary)' }} />
                  Metadata Auto-Assignment
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', marginBottom: '12px' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                      Assign Correspondent
                    </label>
                    <select
                      className="input"
                      value={ruleForm.assign_correspondent || ''}
                      onChange={(e) =>
                        setRuleForm({
                          ...ruleForm,
                          assign_correspondent: e.target.value ? Number(e.target.value) : null,
                        })
                      }
                      style={{ width: '100%', fontSize: '0.82rem' }}
                    >
                      <option value="">(None)</option>
                      {correspondents.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div>
                    <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                      Assign Document Type
                    </label>
                    <select
                      className="input"
                      value={ruleForm.assign_document_type || ''}
                      onChange={(e) =>
                        setRuleForm({
                          ...ruleForm,
                          assign_document_type: e.target.value ? Number(e.target.value) : null,
                        })
                      }
                      style={{ width: '100%', fontSize: '0.82rem' }}
                    >
                      <option value="">(None)</option>
                      {documentTypes.map((dt) => (
                        <option key={dt.id} value={dt.id}>
                          {dt.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                <div>
                  <label style={{ display: 'block', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '6px' }}>
                    Assign Tags
                  </label>
                  <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                    {tags.map((tag) => {
                      const selected = (ruleForm.assign_tags || []).includes(tag.id)
                      return (
                        <button
                          key={tag.id}
                          type="button"
                          onClick={() => {
                            const current = ruleForm.assign_tags || []
                            setRuleForm({
                              ...ruleForm,
                              assign_tags: selected ? current.filter((id: number) => id !== tag.id) : [...current, tag.id],
                            })
                          }}
                          style={{
                            padding: '4px 10px',
                            borderRadius: 'var(--radius-full)',
                            fontSize: '0.78rem',
                            fontWeight: 600,
                            cursor: 'pointer',
                            border: `1px solid ${selected ? tag.color || 'var(--accent-primary)' : 'var(--border-subtle)'}`,
                            backgroundColor: selected ? (tag.color ? `${tag.color}33` : 'rgba(99,102,241,0.25)') : 'transparent',
                            color: selected ? (tag.color || 'var(--accent-primary)') : 'var(--text-muted)',
                            transition: 'all 0.15s ease',
                          }}
                        >
                          {tag.name}
                        </button>
                      )
                    })}
                  </div>
                </div>
              </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '10px' }}>
              <button type="button" className="btn btn-secondary" onClick={() => setRuleModalOpen(false)}>
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  if (editingRule) {
                    updateRuleMutation.mutate({ id: editingRule.id, data: ruleForm })
                  } else {
                    createRuleMutation.mutate(ruleForm)
                  }
                }}
                disabled={!ruleForm.name || !ruleForm.account}
              >
                {editingRule ? 'Save Changes' : 'Create Mail Rule'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
