import { useState, useMemo, useEffect } from 'react'
import {
  TableProperties,
  FileSpreadsheet,
  Download,
  CheckCircle2,
  AlertTriangle,
  Play,
  Plus,
  Printer,
  Eye,
  Check,
  SlidersHorizontal,
  ArrowRight,
  ShieldCheck,
  Zap,
  Trash2,
  Sparkles,
  X,
} from 'lucide-react'
import { useExtractionSchemas, useExtractionBatches, useBatchGrid } from '../api/useExtraction'
import { useDocuments } from '../api/useDocuments'
import type { ExtractionColumn, ExtractionRow, ExtractionCell } from '../types'

const SCHEMA_PRESETS: Record<string, { name: string; desc: string; cols: ExtractionColumn[] }> = {
  invoices: {
    name: 'Vendor Invoices & Bills',
    desc: 'Commercial vendor bills, accounts payable, receipts',
    cols: [
      { key: 'invoice_no', label: 'Invoice Number', type: 'string', required: true, hints: ['Invoice #', 'Inv No', 'Bill #'] },
      { key: 'vendor', label: 'Vendor Name', type: 'string', required: true, hints: ['Vendor', 'Supplier', 'From'] },
      { key: 'invoice_date', label: 'Invoice Date', type: 'date', required: true, hints: ['Date', 'Bill Date'] },
      { key: 'subtotal', label: 'Subtotal', type: 'monetary', required: false, hints: ['Subtotal', 'Net'] },
      { key: 'tax_amount', label: 'Tax Amount', type: 'monetary', required: false, hints: ['Tax', 'VAT', 'GST'] },
      { key: 'total_amount', label: 'Total Amount', type: 'monetary', required: true, hints: ['Total', 'Amount Due'] },
    ],
  },
  hr: {
    name: 'HR & Employee Records',
    desc: 'Personnel files, employee onboarding, identity records',
    cols: [
      { key: 'full_name', label: 'Employee Name', type: 'string', required: true, hints: ['Full Name', 'Employee', 'Staff'] },
      { key: 'staff_id', label: 'Staff ID', type: 'string', required: true, hints: ['Staff ID', 'Badge #', 'Emp ID'] },
      { key: 'hire_date', label: 'Hire Date', type: 'date', required: false, hints: ['Hire Date', 'Joined'] },
      { key: 'department', label: 'Department', type: 'string', required: false, hints: ['Department', 'Dept'] },
      { key: 'job_title', label: 'Job Title', type: 'string', required: false, hints: ['Title', 'Role', 'Position'] },
      { key: 'salary', label: 'Base Salary', type: 'monetary', required: false, hints: ['Salary', 'Wage', 'Pay'] },
    ],
  },
  medical: {
    name: 'Medical & Patient Intake',
    desc: 'Health intake forms, chart summaries, clinic records',
    cols: [
      { key: 'patient_name', label: 'Patient Name', type: 'string', required: true, hints: ['Patient', 'Client Name'] },
      { key: 'mrn', label: 'Medical Record #', type: 'string', required: true, hints: ['MRN', 'Chart #', 'Patient ID'] },
      { key: 'dob', label: 'Date of Birth', type: 'date', required: true, hints: ['DOB', 'Birth Date'] },
      { key: 'provider', label: 'Physician / Provider', type: 'string', required: false, hints: ['Doctor', 'Physician'] },
      { key: 'service_date', label: 'Visit Date', type: 'date', required: false, hints: ['Date of Service', 'Visit Date'] },
      { key: 'copay', label: 'Copay / Fee', type: 'monetary', required: false, hints: ['Copay', 'Fee Due'] },
    ],
  },
  logistics: {
    name: 'Logistics & Waybills',
    desc: 'Bills of lading, delivery receipts, cargo manifests',
    cols: [
      { key: 'tracking_no', label: 'Tracking / Waybill #', type: 'string', required: true, hints: ['Tracking', 'Waybill', 'BOL'] },
      { key: 'shipper', label: 'Shipper / Origin', type: 'string', required: true, hints: ['Shipper', 'From', 'Sender'] },
      { key: 'recipient', label: 'Recipient / Consignee', type: 'string', required: true, hints: ['To', 'Consignee', 'Recipient'] },
      { key: 'ship_date', label: 'Ship Date', type: 'date', required: false, hints: ['Ship Date', 'Dispatched'] },
      { key: 'weight_lbs', label: 'Weight (lbs)', type: 'string', required: false, hints: ['Weight', 'Gross Wt'] },
      { key: 'shipping_fee', label: 'Freight Fee', type: 'monetary', required: false, hints: ['Freight', 'Shipping Fee', 'Total Charge'] },
    ],
  },
}

export function DataEntry() {
  const [activeTab, setActiveTab] = useState<'review' | 'schemas' | 'ingest'>('review')
  const { schemas, createSchema, deleteSchema } = useExtractionSchemas()
  const { batches, reload: reloadBatches, createBatch } = useExtractionBatches()
  const { documents } = useDocuments()

  // Selected batch for review
  const [selectedBatchId, setSelectedBatchId] = useState<number | null>(null)

  // Auto-select first batch when loaded
  useEffect(() => {
    if (!selectedBatchId && batches.length > 0) {
      setSelectedBatchId(batches[0].id)
    }
  }, [batches, selectedBatchId])

  const { gridData, updateCell, approveRecord, triggerProcess } = useBatchGrid(selectedBatchId)

  // Active cell and row for click-to-source inspection
  const [selectedRowId, setSelectedRowId] = useState<number | null>(null)
  const [selectedCellKey, setSelectedCellKey] = useState<string | null>(null)
  const [editingCellId, setEditingCellId] = useState<number | null>(null)
  const [editingValue, setEditingValue] = useState<string>('')
  const [filterMode, setFilterMode] = useState<'all' | 'flagged' | 'approved'>('all')

  // Schema creation modal state
  const [isCreatingSchema, setIsCreatingSchema] = useState(false)
  const [newSchemaName, setNewSchemaName] = useState('')
  const [newSchemaDesc, setNewSchemaDesc] = useState('')
  const [newSchemaCols, setNewSchemaCols] = useState<ExtractionColumn[]>([
    { key: 'invoice_no', label: 'Invoice Number', type: 'string', required: true, hints: ['Invoice #', 'Inv No'] },
    { key: 'vendor', label: 'Vendor Name', type: 'string', required: true, hints: ['Vendor', 'Supplier', 'From'] },
    { key: 'invoice_date', label: 'Invoice Date', type: 'date', required: true, hints: ['Date', 'Bill Date'] },
    { key: 'total_amount', label: 'Total Amount', type: 'monetary', required: true, hints: ['Total', 'Gross Amount'] },
  ])

  const handleAddColumn = () => {
    const count = newSchemaCols.length + 1
    setNewSchemaCols((prev) => [
      ...prev,
      {
        key: `field_${count}`,
        label: `Field ${count}`,
        type: 'string',
        required: false,
        hints: [],
      },
    ])
  }

  const handleUpdateColumn = (index: number, updates: Partial<ExtractionColumn>) => {
    setNewSchemaCols((prev) => {
      const next = [...prev]
      const old = next[index]
      let autoKey = updates.key !== undefined ? updates.key : old.key

      // If user edits label and hasn't manually diverged the key, keep key synced
      if (updates.label !== undefined && updates.key === undefined) {
        const slug = updates.label.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '')
        if (slug) autoKey = slug
      }

      next[index] = {
        ...old,
        ...updates,
        key: autoKey,
      }
      return next
    })
  }

  const handleRemoveColumn = (index: number) => {
    if (newSchemaCols.length <= 1) return
    setNewSchemaCols((prev) => prev.filter((_, i) => i !== index))
  }

  const handleApplyPreset = (key: string) => {
    const p = SCHEMA_PRESETS[key]
    if (!p) return
    setNewSchemaName(p.name)
    setNewSchemaDesc(p.desc)
    setNewSchemaCols([...p.cols])
  }

  // Ingest / batch creation state
  const [newBatchTitle, setNewBatchTitle] = useState('')
  const [newBatchSchemaId, setNewBatchSchemaId] = useState<number>(schemas[0]?.id || 1)
  const [selectedDocIds, setSelectedDocIds] = useState<number[]>([])

  // Filtered rows for grid
  const displayedRows = useMemo(() => {
    if (!gridData?.rows) return []
    if (filterMode === 'flagged') {
      return gridData.rows.filter(
        (r) => r.status === 'needs_review' || Object.values(r.cells).some((c) => c.confidence < 0.9 || c.validation_error)
      )
    }
    if (filterMode === 'approved') {
      return gridData.rows.filter((r) => r.status === 'approved')
    }
    return gridData.rows
  }, [gridData, filterMode])

  // Active selected row
  const activeRow: ExtractionRow | undefined = useMemo(() => {
    if (!gridData?.rows) return undefined
    if (selectedRowId !== null) {
      return gridData.rows.find((r) => r.record_id === selectedRowId) || gridData.rows[0]
    }
    return gridData.rows[0]
  }, [gridData, selectedRowId])

  // Active selected cell
  const activeCell: ExtractionCell | undefined = useMemo(() => {
    if (!activeRow) return undefined
    if (selectedCellKey && activeRow.cells[selectedCellKey]) {
      return activeRow.cells[selectedCellKey]
    }
    const firstKey = Object.keys(activeRow.cells)[0]
    return firstKey ? activeRow.cells[firstKey] : undefined
  }, [activeRow, selectedCellKey])

  const handleCellClick = (recordId: number, columnKey: string, _cell: ExtractionCell) => {
    setSelectedRowId(recordId)
    setSelectedCellKey(columnKey)
  }

  const handleCellDoubleClick = (cell: ExtractionCell) => {
    setEditingCellId(cell.id)
    setEditingValue(cell.value || cell.raw_value || '')
  }

  const handleCellSave = async (cellId: number) => {
    if (editingCellId === cellId) {
      await updateCell(cellId, editingValue)
      setEditingCellId(null)
    }
  }

  const handleExportExcel = () => {
    if (!selectedBatchId) return
    window.open(`/api/extraction/batches/${selectedBatchId}/export/`, '_blank')
  }

  const handleCreateNewBatch = async () => {
    if (!newBatchTitle) return
    await createBatch({
      schema_id: newBatchSchemaId,
      title: newBatchTitle,
      document_ids: selectedDocIds.length > 0 ? selectedDocIds : documents?.slice(0, 15).map((d) => d.id),
    })
    setNewBatchTitle('')
    setSelectedDocIds([])
    await reloadBatches()
    setActiveTab('review')
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px', width: '100%', boxSizing: 'border-box' }}>
      {/* Top Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: '16px' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <div
              style={{
                width: '36px',
                height: '36px',
                borderRadius: '10px',
                background: 'rgba(16, 185, 129, 0.12)',
                border: '1px solid rgba(16, 185, 129, 0.3)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: '#10b981',
              }}
            >
              <TableProperties size={20} />
            </div>
            <div>
              <h1 style={{ margin: 0, fontSize: '1.6rem', fontWeight: 700, color: 'var(--text-primary)', letterSpacing: '-0.02em' }}>
                Data Entry & IDP Workspace
              </h1>
              <p style={{ margin: '3px 0 0', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                Batch digitize physical scans, extract structured columns, verify with click-to-source provenance, and export clean Excel workbooks.
              </p>
            </div>
          </div>
        </div>

        {/* Tab Switcher */}
        <div
          style={{
            display: 'flex',
            backgroundColor: 'var(--bg-secondary)',
            padding: '4px',
            borderRadius: '12px',
            border: '1px solid var(--border-color)',
            gap: '4px',
          }}
        >
          <button
            onClick={() => setActiveTab('review')}
            style={{
              padding: '8px 16px',
              borderRadius: '8px',
              border: 'none',
              background: activeTab === 'review' ? 'var(--accent-primary)' : 'transparent',
              color: activeTab === 'review' ? '#fff' : 'var(--text-secondary)',
              fontWeight: 600,
              fontSize: '0.85rem',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              transition: 'all 0.15s ease',
            }}
          >
            <FileSpreadsheet size={16} />
            Review Grid
          </button>
          <button
            onClick={() => setActiveTab('schemas')}
            style={{
              padding: '8px 16px',
              borderRadius: '8px',
              border: 'none',
              background: activeTab === 'schemas' ? 'var(--accent-primary)' : 'transparent',
              color: activeTab === 'schemas' ? '#fff' : 'var(--text-secondary)',
              fontWeight: 600,
              fontSize: '0.85rem',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              transition: 'all 0.15s ease',
            }}
          >
            <SlidersHorizontal size={16} />
            Target Schemas ({schemas.length})
          </button>
          <button
            onClick={() => setActiveTab('ingest')}
            style={{
              padding: '8px 16px',
              borderRadius: '8px',
              border: 'none',
              background: activeTab === 'ingest' ? 'var(--accent-primary)' : 'transparent',
              color: activeTab === 'ingest' ? '#fff' : 'var(--text-secondary)',
              fontWeight: 600,
              fontSize: '0.85rem',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              transition: 'all 0.15s ease',
            }}
          >
            <Printer size={16} />
            Scan & Ingest
          </button>
        </div>
      </div>

      {/* TAB 1: REVIEW GRID WORKSPACE */}
      {activeTab === 'review' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {/* Controls Bar */}
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              flexWrap: 'wrap',
              gap: '12px',
              backgroundColor: 'var(--bg-secondary)',
              padding: '12px 18px',
              borderRadius: '12px',
              border: '1px solid var(--border-color)',
            }}
          >
            {/* Batch Selector & Status */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <span style={{ fontSize: '0.82rem', fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase' }}>
                Batch:
              </span>
              <select
                value={selectedBatchId || ''}
                onChange={(e) => setSelectedBatchId(Number(e.target.value))}
                style={{
                  backgroundColor: 'var(--bg-tertiary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '8px',
                  padding: '6px 12px',
                  fontSize: '0.88rem',
                  fontWeight: 600,
                  outline: 'none',
                  cursor: 'pointer',
                }}
              >
                {batches.map((b) => (
                  <option key={b.id} value={b.id}>
                    #{b.id} · {b.title} ({b.processed_documents}/{b.total_documents} docs)
                  </option>
                ))}
              </select>

              {gridData && (
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginLeft: '6px' }}>
                  <span
                    style={{
                      padding: '3px 9px',
                      borderRadius: '12px',
                      fontSize: '0.74rem',
                      fontWeight: 700,
                      backgroundColor: gridData.status === 'completed' ? 'rgba(16, 185, 129, 0.15)' : 'rgba(245, 158, 11, 0.15)',
                      color: gridData.status === 'completed' ? '#10b981' : '#f59e0b',
                      border: `1px solid ${gridData.status === 'completed' ? 'rgba(16, 185, 129, 0.3)' : 'rgba(245, 158, 11, 0.3)'}`,
                    }}
                  >
                    {gridData.status.toUpperCase()}
                  </span>
                  {gridData.flagged_fields_count > 0 && (
                    <span
                      style={{
                        padding: '3px 9px',
                        borderRadius: '12px',
                        fontSize: '0.74rem',
                        fontWeight: 700,
                        backgroundColor: 'rgba(239, 68, 68, 0.15)',
                        color: '#ef4444',
                        border: '1px solid rgba(239, 68, 68, 0.3)',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '4px',
                      }}
                    >
                      <AlertTriangle size={12} />
                      {gridData.flagged_fields_count} Flagged
                    </span>
                  )}
                </div>
              )}
            </div>

            {/* Filter Buttons & Actions */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <div
                style={{
                  display: 'flex',
                  backgroundColor: 'var(--bg-tertiary)',
                  padding: '2px',
                  borderRadius: '8px',
                  border: '1px solid var(--border-color)',
                }}
              >
                <button
                  onClick={() => setFilterMode('all')}
                  style={{
                    padding: '5px 12px',
                    borderRadius: '6px',
                    border: 'none',
                    background: filterMode === 'all' ? 'var(--accent-primary)' : 'transparent',
                    color: filterMode === 'all' ? '#fff' : 'var(--text-secondary)',
                    fontSize: '0.78rem',
                    fontWeight: 600,
                    cursor: 'pointer',
                  }}
                >
                  All ({gridData?.rows.length || 0})
                </button>
                <button
                  onClick={() => setFilterMode('flagged')}
                  style={{
                    padding: '5px 12px',
                    borderRadius: '6px',
                    border: 'none',
                    background: filterMode === 'flagged' ? '#ef4444' : 'transparent',
                    color: filterMode === 'flagged' ? '#fff' : 'var(--text-secondary)',
                    fontSize: '0.78rem',
                    fontWeight: 600,
                    cursor: 'pointer',
                  }}
                >
                  Needs Review
                </button>
                <button
                  onClick={() => setFilterMode('approved')}
                  style={{
                    padding: '5px 12px',
                    borderRadius: '6px',
                    border: 'none',
                    background: filterMode === 'approved' ? '#10b981' : 'transparent',
                    color: filterMode === 'approved' ? '#fff' : 'var(--text-secondary)',
                    fontSize: '0.78rem',
                    fontWeight: 600,
                    cursor: 'pointer',
                  }}
                >
                  Approved
                </button>
              </div>

              <button
                onClick={triggerProcess}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '6px 12px',
                  borderRadius: '8px',
                  border: '1px solid var(--border-color)',
                  backgroundColor: 'var(--bg-tertiary)',
                  color: 'var(--text-primary)',
                  fontSize: '0.82rem',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                <Play size={14} color="#10b981" />
                Run Extraction
              </button>

              <button
                onClick={handleExportExcel}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '6px 14px',
                  borderRadius: '8px',
                  border: 'none',
                  backgroundColor: '#10b981',
                  color: '#ffffff',
                  fontSize: '0.82rem',
                  fontWeight: 700,
                  cursor: 'pointer',
                  boxShadow: '0 2px 8px rgba(16, 185, 129, 0.3)',
                }}
              >
                <Download size={14} />
                Export to Excel (.xlsx)
              </button>
            </div>
          </div>

          {/* SPLIT SCREEN WORKSPACE: Review Grid (Left) + Document Canvas (Right) */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'minmax(600px, 1.3fr) minmax(380px, 1fr)',
              gap: '18px',
              minHeight: '680px',
            }}
          >
            {/* LEFT PANEL: Virtualized Data Review Grid */}
            <div
              style={{
                backgroundColor: 'var(--bg-secondary)',
                borderRadius: '12px',
                border: '1px solid var(--border-color)',
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  padding: '12px 18px',
                  borderBottom: '1px solid var(--border-color)',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  backgroundColor: 'var(--bg-tertiary)',
                }}
              >
                <span style={{ fontSize: '0.85rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                  Extracted Records ({displayedRows.length})
                </span>
                <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)' }}>
                  Click a cell to highlight source · Double click to edit · Press [Enter] to save
                </span>
              </div>

              {/* Grid Container */}
              <div style={{ flex: 1, overflowX: 'auto', overflowY: 'auto', position: 'relative' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.84rem' }}>
                  <thead>
                    <tr style={{ backgroundColor: 'var(--bg-primary)', borderBottom: '1px solid var(--border-color)' }}>
                      <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 600, fontSize: '0.75rem', width: '50px' }}>
                        #
                      </th>
                      <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 600, fontSize: '0.75rem', minWidth: '140px' }}>
                        Source File
                      </th>
                      {gridData?.columns.map((col) => (
                        <th
                          key={col.key}
                          style={{
                            padding: '10px 14px',
                            color: 'var(--text-muted)',
                            fontWeight: 600,
                            fontSize: '0.75rem',
                            minWidth: '130px',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {col.label} {col.required && <span style={{ color: '#ef4444' }}>*</span>}
                        </th>
                      ))}
                      <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 600, fontSize: '0.75rem', width: '80px', textAlign: 'center' }}>
                        Action
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {displayedRows.map((row, idx) => {
                      const isRowActive = activeRow?.record_id === row.record_id
                      return (
                        <tr
                          key={row.record_id}
                          style={{
                            backgroundColor: isRowActive ? 'var(--accent-soft)' : idx % 2 === 0 ? 'transparent' : 'rgba(255, 255, 255, 0.015)',
                            borderBottom: '1px solid var(--border-color)',
                            cursor: 'pointer',
                            transition: 'background-color 0.1s ease',
                          }}
                          onClick={() => setSelectedRowId(row.record_id)}
                        >
                          <td style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 600 }}>
                            {idx + 1}
                          </td>
                          <td
                            style={{
                              padding: '10px 14px',
                              fontWeight: 600,
                              color: 'var(--text-primary)',
                              whiteSpace: 'nowrap',
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              maxWidth: '180px',
                            }}
                            title={row.filename}
                          >
                            {row.filename}
                          </td>

                          {/* Dynamic Schema Columns */}
                          {gridData?.columns.map((col) => {
                            const cell = row.cells[col.key] || {
                              id: 0,
                              value: null,
                              confidence: 0,
                              data_type: col.type,
                              page_number: 1,
                              is_reviewed: false,
                            }
                            const isCellActive = isRowActive && selectedCellKey === col.key
                            const isEditing = editingCellId === cell.id

                            const isFlagged = cell.confidence < 0.9 || cell.validation_error
                            const isApproved = cell.is_reviewed

                            return (
                              <td
                                key={col.key}
                                onClick={(e) => {
                                  e.stopPropagation()
                                  handleCellClick(row.record_id, col.key, cell)
                                }}
                                onDoubleClick={(e) => {
                                  e.stopPropagation()
                                  handleCellDoubleClick(cell)
                                }}
                                style={{
                                  padding: '8px 12px',
                                  borderRight: '1px solid rgba(255, 255, 255, 0.04)',
                                  backgroundColor: isCellActive
                                    ? 'rgba(59, 130, 246, 0.15)'
                                    : isFlagged && !isApproved
                                    ? 'rgba(239, 68, 68, 0.08)'
                                    : 'transparent',
                                  outline: isCellActive ? '1px solid var(--accent-primary)' : 'none',
                                }}
                              >
                                {isEditing ? (
                                  <input
                                    autoFocus
                                    value={editingValue}
                                    onChange={(e) => setEditingValue(e.target.value)}
                                    onBlur={() => handleCellSave(cell.id)}
                                    onKeyDown={(e) => {
                                      if (e.key === 'Enter') handleCellSave(cell.id)
                                      if (e.key === 'Escape') setEditingCellId(null)
                                    }}
                                    style={{
                                      width: '100%',
                                      backgroundColor: 'var(--bg-primary)',
                                      color: 'var(--text-primary)',
                                      border: '1px solid var(--accent-primary)',
                                      borderRadius: '4px',
                                      padding: '4px 6px',
                                      fontSize: '0.84rem',
                                      outline: 'none',
                                    }}
                                  />
                                ) : (
                                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '6px' }}>
                                    <span
                                      style={{
                                        color: cell.value ? 'var(--text-primary)' : 'var(--text-muted)',
                                        fontStyle: cell.value ? 'normal' : 'italic',
                                        fontWeight: cell.value ? 500 : 400,
                                      }}
                                    >
                                      {cell.value || 'Empty'}
                                    </span>

                                    {/* Confidence / Review Indicator */}
                                    {isFlagged && !isApproved && (
                                      <span
                                        title={cell.validation_error || `Low confidence (${Math.round(cell.confidence * 100)}%)`}
                                        style={{
                                          width: '7px',
                                          height: '7px',
                                          borderRadius: '50%',
                                          backgroundColor: '#ef4444',
                                          flexShrink: 0,
                                        }}
                                      />
                                    )}
                                    {isApproved && (
                                      <Check size={12} color="#10b981" style={{ flexShrink: 0 }} />
                                    )}
                                  </div>
                                )}
                              </td>
                            )
                          })}

                          {/* Row Approval Action */}
                          <td style={{ padding: '8px 12px', textAlign: 'center' }}>
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                approveRecord(row.record_id)
                              }}
                              title="Approve Row"
                              style={{
                                border: 'none',
                                background: row.status === 'approved' ? 'rgba(16, 185, 129, 0.15)' : 'var(--bg-tertiary)',
                                color: row.status === 'approved' ? '#10b981' : 'var(--text-muted)',
                                borderRadius: '6px',
                                padding: '4px 8px',
                                cursor: 'pointer',
                              }}
                            >
                              <CheckCircle2 size={16} />
                            </button>
                          </td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            </div>

            {/* RIGHT PANEL: Synchronized Provenance Canvas & Bounding Box Viewer */}
            <div
              style={{
                backgroundColor: 'var(--bg-secondary)',
                borderRadius: '12px',
                border: '1px solid var(--border-color)',
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  padding: '12px 18px',
                  borderBottom: '1px solid var(--border-color)',
                  backgroundColor: 'var(--bg-tertiary)',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <Eye size={16} color="var(--accent-primary)" />
                  <span style={{ fontSize: '0.85rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                    Source Verification & Bounding Box
                  </span>
                </div>
                {activeRow && (
                  <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)' }}>
                    {activeRow.filename} · Page 1 of 1
                  </span>
                )}
              </div>

              {/* Provenance Document Canvas */}
              <div
                style={{
                  flex: 1,
                  display: 'flex',
                  flexDirection: 'column',
                  padding: '20px',
                  backgroundColor: '#0b1120',
                  alignItems: 'center',
                  justifyContent: 'center',
                  position: 'relative',
                  overflow: 'auto',
                }}
              >
                {activeRow ? (
                  <div
                    style={{
                      width: '360px',
                      minHeight: '480px',
                      backgroundColor: '#ffffff',
                      borderRadius: '6px',
                      boxShadow: '0 8px 30px rgba(0, 0, 0, 0.5)',
                      position: 'relative',
                      overflow: 'hidden',
                      display: 'flex',
                      flexDirection: 'column',
                    }}
                  >
                    {activeRow.document_id ? (
                      <div style={{ position: 'relative', width: '100%', height: '100%' }}>
                        <img
                          src={`/api/documents/${activeRow.document_id}/preview/`}
                          alt={`Preview of ${activeRow.filename}`}
                          style={{
                            width: '100%',
                            height: 'auto',
                            display: 'block',
                            backgroundColor: '#fff',
                          }}
                          onError={(e) => {
                            const target = e.currentTarget
                            target.onerror = null
                            target.src = `/api/documents/${activeRow.document_id}/thumb/`
                          }}
                        />
                        {/* Animated Highlighting Bounding Box on top of the real document */}
                        {activeCell?.bbox && (
                          <div
                            style={{
                              position: 'absolute',
                              left: `${activeCell.bbox[0] * 100}%`,
                              top: `${activeCell.bbox[1] * 100}%`,
                              width: `${(activeCell.bbox[2] - activeCell.bbox[0]) * 100}%`,
                              height: `${(activeCell.bbox[3] - activeCell.bbox[1]) * 100}%`,
                              border: '2px solid #3b82f6',
                              backgroundColor: 'rgba(59, 130, 246, 0.3)',
                              borderRadius: '4px',
                              boxShadow: '0 0 12px rgba(59, 130, 246, 0.7)',
                              transition: 'all 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
                              pointerEvents: 'none',
                            }}
                          />
                        )}
                      </div>
                    ) : (
                      <div style={{ padding: '24px 20px', color: '#1e293b', fontSize: '0.72rem', fontFamily: 'monospace' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', borderBottom: '2px solid #0f172a', paddingBottom: '8px', marginBottom: '14px' }}>
                          <span style={{ fontWeight: 800, fontSize: '0.9rem', color: '#0f172a' }}>OFFICIAL RECORD</span>
                          <span style={{ color: '#64748b' }}>DOC-{activeRow.record_id}</span>
                        </div>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', opacity: 0.85 }}>
                          <div>FILE: {activeRow.filename}</div>
                          <div>STATUS: {activeRow.status}</div>
                          <div>CONFIDENCE: {Math.round(activeRow.confidence_score * 100)}%</div>
                        </div>
                      </div>
                    )}
                  </div>
                ) : (
                  <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>Select a document row to view source provenance</div>
                )}

                {/* Provenance Metadata Card */}
                {activeCell && (
                  <div
                    style={{
                      marginTop: '16px',
                      width: '100%',
                      maxWidth: '380px',
                      backgroundColor: 'var(--bg-secondary)',
                      borderRadius: '8px',
                      padding: '12px 16px',
                      border: '1px solid var(--border-color)',
                      boxSizing: 'border-box',
                    }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                      <span style={{ fontSize: '0.78rem', fontWeight: 700, color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                        Active Field: {selectedCellKey}
                      </span>
                      <span
                        style={{
                          fontSize: '0.72rem',
                          fontWeight: 700,
                          padding: '2px 8px',
                          borderRadius: '10px',
                          backgroundColor: activeCell.confidence >= 0.9 ? 'rgba(16, 185, 129, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                          color: activeCell.confidence >= 0.9 ? '#10b981' : '#ef4444',
                        }}
                      >
                        Confidence: {Math.round(activeCell.confidence * 100)}%
                      </span>
                    </div>

                    <div style={{ fontSize: '0.88rem', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '4px' }}>
                      Value: <span style={{ color: 'var(--accent-primary)' }}>{activeCell.value || 'None'}</span>
                    </div>

                    {activeCell.validation_error && (
                      <div style={{ fontSize: '0.75rem', color: '#ef4444', marginTop: '4px' }}>
                        ⚠️ {activeCell.validation_error}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: SCHEMA DESIGNER */}
      {activeTab === 'schemas' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <h2 style={{ margin: 0, fontSize: '1.2rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                Target Extraction Schemas
              </h2>
              <p style={{ margin: '4px 0 0', fontSize: '0.82rem', color: 'var(--text-secondary)' }}>
                Configure the destination Excel columns, validation patterns, and anchor hints for your documents.
              </p>
            </div>
            <button
              onClick={() => setIsCreatingSchema(true)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                padding: '8px 16px',
                borderRadius: '8px',
                border: 'none',
                backgroundColor: 'var(--accent-primary)',
                color: '#fff',
                fontSize: '0.84rem',
                fontWeight: 600,
                cursor: 'pointer',
              }}
            >
              <Plus size={16} />
              New Schema
            </button>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))', gap: '16px' }}>
            {schemas.map((schema) => (
              <div
                key={schema.id}
                style={{
                  backgroundColor: 'var(--bg-secondary)',
                  borderRadius: '12px',
                  border: '1px solid var(--border-color)',
                  padding: '20px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '14px',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                  <div>
                    <h3 style={{ margin: 0, fontSize: '1.05rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                      {schema.name}
                    </h3>
                    <p style={{ margin: '4px 0 0', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      {schema.description || 'Custom office extraction schema'}
                    </p>
                  </div>
                  <span
                    style={{
                      padding: '3px 8px',
                      borderRadius: '10px',
                      fontSize: '0.72rem',
                      fontWeight: 700,
                      backgroundColor: 'var(--bg-tertiary)',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {schema.columns.length} Columns
                  </span>
                </div>

                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                  {schema.columns.map((c) => (
                    <span
                      key={c.key}
                      style={{
                        padding: '3px 8px',
                        borderRadius: '6px',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                        backgroundColor: 'var(--bg-tertiary)',
                        color: 'var(--text-primary)',
                        border: '1px solid var(--border-color)',
                      }}
                    >
                      {c.label} ({c.type})
                    </span>
                  ))}
                </div>

                <div style={{ display: 'flex', justifyContent: 'flex-end', alignItems: 'center', gap: '8px', marginTop: 'auto', paddingTop: '8px' }}>
                  <button
                    onClick={async (e) => {
                      e.stopPropagation()
                      if (confirm(`Delete schema "${schema.name}"?`)) {
                        await deleteSchema(schema.id)
                      }
                    }}
                    title="Delete schema"
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                      padding: '6px 8px',
                      borderRadius: '6px',
                      border: '1px solid rgba(239, 68, 68, 0.3)',
                      backgroundColor: 'rgba(239, 68, 68, 0.08)',
                      color: '#ef4444',
                      fontSize: '0.78rem',
                      cursor: 'pointer',
                    }}
                  >
                    <Trash2 size={13} />
                  </button>
                  <button
                    onClick={() => {
                      setNewBatchSchemaId(schema.id)
                      setNewBatchTitle(`${schema.name} Batch - ${new Date().toLocaleDateString()}`)
                      setActiveTab('ingest')
                    }}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                      padding: '6px 12px',
                      borderRadius: '6px',
                      border: '1px solid var(--border-color)',
                      backgroundColor: 'var(--bg-tertiary)',
                      color: 'var(--accent-primary)',
                      fontSize: '0.78rem',
                      fontWeight: 600,
                      cursor: 'pointer',
                    }}
                  >
                    Use in Batch <ArrowRight size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>

          {/* Create Schema Modal with Dynamic Field Builder */}
          {isCreatingSchema && (
            <div
              style={{
                position: 'fixed',
                inset: 0,
                backgroundColor: 'rgba(0, 0, 0, 0.75)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                zIndex: 1000,
                padding: '20px',
              }}
            >
              <div
                style={{
                  backgroundColor: 'var(--bg-secondary)',
                  borderRadius: '16px',
                  border: '1px solid var(--border-color)',
                  padding: '24px',
                  width: '740px',
                  maxWidth: '95vw',
                  maxHeight: '90vh',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '16px',
                  boxShadow: '0 20px 40px rgba(0, 0, 0, 0.5)',
                  overflowY: 'auto',
                }}
              >
                {/* Modal Header */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                  <div>
                    <h3 style={{ margin: 0, fontSize: '1.25rem', fontWeight: 800, color: 'var(--text-primary)' }}>
                      Create Extraction Schema
                    </h3>
                    <p style={{ margin: '4px 0 0', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      Define target spreadsheet columns and keywords for automated document data entry
                    </p>
                  </div>
                  <button
                    onClick={() => setIsCreatingSchema(false)}
                    style={{
                      border: 'none',
                      background: 'transparent',
                      color: 'var(--text-secondary)',
                      cursor: 'pointer',
                      padding: '4px',
                    }}
                  >
                    <X size={20} />
                  </button>
                </div>

                {/* Quick Preset Templates */}
                <div style={{ backgroundColor: 'var(--bg-tertiary)', padding: '12px', borderRadius: '10px', border: '1px solid var(--border-color)' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '8px' }}>
                    <Sparkles size={14} color="var(--accent-primary)" />
                    <span style={{ fontSize: '0.74rem', fontWeight: 700, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                      Quick Starter Presets
                    </span>
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                    {Object.entries(SCHEMA_PRESETS).map(([k, p]) => (
                      <button
                        key={k}
                        type="button"
                        onClick={() => handleApplyPreset(k)}
                        style={{
                          padding: '5px 12px',
                          borderRadius: '8px',
                          border: '1px solid var(--border-color)',
                          backgroundColor: 'var(--bg-secondary)',
                          color: 'var(--text-primary)',
                          fontSize: '0.78rem',
                          fontWeight: 600,
                          cursor: 'pointer',
                          display: 'flex',
                          alignItems: 'center',
                          gap: '6px',
                        }}
                      >
                        {p.name}
                      </button>
                    ))}
                    <button
                      type="button"
                      onClick={() => {
                        setNewSchemaName('')
                        setNewSchemaDesc('')
                        setNewSchemaCols([{ key: 'field_1', label: 'Field 1', type: 'string', required: true, hints: [] }])
                      }}
                      style={{
                        padding: '5px 10px',
                        borderRadius: '8px',
                        border: '1px dashed var(--border-color)',
                        backgroundColor: 'transparent',
                        color: 'var(--text-secondary)',
                        fontSize: '0.78rem',
                        cursor: 'pointer',
                      }}
                    >
                      Clear to Blank
                    </button>
                  </div>
                </div>

                {/* Schema Name & Description */}
                <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1fr', gap: '12px' }}>
                  <div>
                    <label style={{ fontSize: '0.72rem', fontWeight: 700, color: 'var(--text-muted)', textTransform: 'uppercase' }}>
                      Schema Name *
                    </label>
                    <input
                      placeholder="e.g. Medical Intake Records, Bill of Lading"
                      value={newSchemaName}
                      onChange={(e) => setNewSchemaName(e.target.value)}
                      style={{
                        width: '100%',
                        backgroundColor: 'var(--bg-tertiary)',
                        color: 'var(--text-primary)',
                        border: '1px solid var(--border-color)',
                        borderRadius: '8px',
                        padding: '9px 12px',
                        fontSize: '0.85rem',
                        marginTop: '4px',
                        boxSizing: 'border-box',
                        outline: 'none',
                      }}
                    />
                  </div>
                  <div>
                    <label style={{ fontSize: '0.72rem', fontWeight: 700, color: 'var(--text-muted)', textTransform: 'uppercase' }}>
                      Description (Optional)
                    </label>
                    <input
                      placeholder="e.g. Clinic check-ins and patient charts"
                      value={newSchemaDesc}
                      onChange={(e) => setNewSchemaDesc(e.target.value)}
                      style={{
                        width: '100%',
                        backgroundColor: 'var(--bg-tertiary)',
                        color: 'var(--text-primary)',
                        border: '1px solid var(--border-color)',
                        borderRadius: '8px',
                        padding: '9px 12px',
                        fontSize: '0.85rem',
                        marginTop: '4px',
                        boxSizing: 'border-box',
                        outline: 'none',
                      }}
                    />
                  </div>
                </div>

                {/* Target Columns & Fields Builder */}
                <div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                    <div>
                      <span style={{ fontSize: '0.75rem', fontWeight: 800, color: 'var(--text-primary)', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                        Columns to Extract ({newSchemaCols.length})
                      </span>
                      <p style={{ margin: '2px 0 0', fontSize: '0.72rem', color: 'var(--text-secondary)' }}>
                        Add fields to extract from scanned documents into spreadsheet columns
                      </p>
                    </div>
                    <button
                      type="button"
                      onClick={handleAddColumn}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '6px',
                        padding: '6px 12px',
                        borderRadius: '8px',
                        border: '1px solid var(--accent-primary)',
                        backgroundColor: 'rgba(59, 130, 246, 0.1)',
                        color: 'var(--accent-primary)',
                        fontSize: '0.78rem',
                        fontWeight: 700,
                        cursor: 'pointer',
                      }}
                    >
                      <Plus size={14} /> Add Column
                    </button>
                  </div>

                  {/* Column List */}
                  <div
                    style={{
                      maxHeight: '260px',
                      overflowY: 'auto',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '8px',
                      paddingRight: '4px',
                    }}
                  >
                    {newSchemaCols.map((col, idx) => (
                      <div
                        key={idx}
                        style={{
                          display: 'grid',
                          gridTemplateColumns: '1.6fr 1.2fr 1.1fr 2fr 50px 32px',
                          gap: '8px',
                          alignItems: 'center',
                          padding: '8px 10px',
                          borderRadius: '8px',
                          backgroundColor: 'var(--bg-tertiary)',
                          border: '1px solid var(--border-color)',
                        }}
                      >
                        {/* Label */}
                        <div>
                          <input
                            placeholder="Field Label (e.g. Patient ID)"
                            value={col.label}
                            onChange={(e) => handleUpdateColumn(idx, { label: e.target.value })}
                            style={{
                              width: '100%',
                              backgroundColor: 'var(--bg-secondary)',
                              color: 'var(--text-primary)',
                              border: '1px solid var(--border-color)',
                              borderRadius: '6px',
                              padding: '6px 8px',
                              fontSize: '0.78rem',
                              boxSizing: 'border-box',
                              outline: 'none',
                            }}
                          />
                        </div>

                        {/* Key */}
                        <div>
                          <input
                            placeholder="key_name"
                            value={col.key}
                            onChange={(e) => handleUpdateColumn(idx, { key: e.target.value })}
                            style={{
                              width: '100%',
                              backgroundColor: 'var(--bg-secondary)',
                              color: 'var(--text-primary)',
                              border: '1px solid var(--border-color)',
                              borderRadius: '6px',
                              padding: '6px 8px',
                              fontSize: '0.75rem',
                              fontFamily: 'monospace',
                              boxSizing: 'border-box',
                              outline: 'none',
                            }}
                          />
                        </div>

                        {/* Type */}
                        <div>
                          <select
                            value={col.type}
                            onChange={(e) => handleUpdateColumn(idx, { type: e.target.value as ExtractionColumn['type'] })}
                            style={{
                              width: '100%',
                              backgroundColor: 'var(--bg-secondary)',
                              color: 'var(--text-primary)',
                              border: '1px solid var(--border-color)',
                              borderRadius: '6px',
                              padding: '6px 4px',
                              fontSize: '0.75rem',
                              boxSizing: 'border-box',
                              outline: 'none',
                            }}
                          >
                            <option value="string">Text / ID</option>
                            <option value="date">Date</option>
                            <option value="monetary">Currency ($)</option>
                            <option value="number">Number (#)</option>
                            <option value="boolean">Boolean</option>
                          </select>
                        </div>

                        {/* Hints */}
                        <div>
                          <input
                            placeholder="Hints: Inv #, Bill, Total"
                            value={col.hints?.join(', ') || ''}
                            onChange={(e) =>
                              handleUpdateColumn(idx, {
                                hints: e.target.value
                                  .split(',')
                                  .map((h) => h.trim())
                                  .filter(Boolean),
                              })
                            }
                            title="Alternative keywords or synonyms on physical documents"
                            style={{
                              width: '100%',
                              backgroundColor: 'var(--bg-secondary)',
                              color: 'var(--text-primary)',
                              border: '1px solid var(--border-color)',
                              borderRadius: '6px',
                              padding: '6px 8px',
                              fontSize: '0.75rem',
                              boxSizing: 'border-box',
                              outline: 'none',
                            }}
                          />
                        </div>

                        {/* Required toggle */}
                        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '3px' }} title="Mark as Required">
                          <input
                            type="checkbox"
                            id={`col_req_${idx}`}
                            checked={col.required}
                            onChange={(e) => handleUpdateColumn(idx, { required: e.target.checked })}
                          />
                          <label htmlFor={`col_req_${idx}`} style={{ fontSize: '0.68rem', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                            Req
                          </label>
                        </div>

                        {/* Delete button */}
                        <button
                          type="button"
                          onClick={() => handleRemoveColumn(idx)}
                          disabled={newSchemaCols.length <= 1}
                          title={newSchemaCols.length <= 1 ? 'Minimum 1 column required' : 'Delete column'}
                          style={{
                            border: 'none',
                            background: 'transparent',
                            color: newSchemaCols.length <= 1 ? 'var(--text-muted)' : '#ef4444',
                            cursor: newSchemaCols.length <= 1 ? 'not-allowed' : 'pointer',
                            padding: '4px',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                          }}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Modal Footer Actions */}
                <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '6px', paddingTop: '10px', borderTop: '1px solid var(--border-color)' }}>
                  <button
                    onClick={() => setIsCreatingSchema(false)}
                    style={{
                      padding: '8px 16px',
                      borderRadius: '8px',
                      border: '1px solid var(--border-color)',
                      backgroundColor: 'transparent',
                      color: 'var(--text-secondary)',
                      fontSize: '0.84rem',
                      cursor: 'pointer',
                    }}
                  >
                    Cancel
                  </button>
                  <button
                    onClick={async () => {
                      if (!newSchemaName.trim() || newSchemaCols.length === 0) return
                      await createSchema({
                        name: newSchemaName.trim(),
                        description: newSchemaDesc.trim(),
                        columns: newSchemaCols,
                      })
                      setIsCreatingSchema(false)
                      setNewSchemaName('')
                      setNewSchemaDesc('')
                    }}
                    disabled={!newSchemaName.trim() || newSchemaCols.length === 0}
                    style={{
                      padding: '8px 20px',
                      borderRadius: '8px',
                      border: 'none',
                      backgroundColor: newSchemaName.trim() ? 'var(--accent-primary)' : 'var(--bg-tertiary)',
                      color: newSchemaName.trim() ? '#fff' : 'var(--text-muted)',
                      fontSize: '0.84rem',
                      fontWeight: 700,
                      cursor: newSchemaName.trim() ? 'pointer' : 'not-allowed',
                      boxShadow: newSchemaName.trim() ? '0 2px 8px rgba(59, 130, 246, 0.3)' : 'none',
                    }}
                  >
                    Save Schema ({newSchemaCols.length} Columns)
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* TAB 3: SCAN & BATCH INGEST */}
      {activeTab === 'ingest' && (
        <div style={{ display: 'grid', gridTemplateColumns: 'minmax(340px, 1fr) minmax(400px, 1.2fr)', gap: '20px' }}>
          {/* Left: Scanner Direct Feeder */}
          <div
            style={{
              backgroundColor: 'var(--bg-secondary)',
              borderRadius: '12px',
              border: '1px solid var(--border-color)',
              padding: '24px',
              display: 'flex',
              flexDirection: 'column',
              gap: '18px',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Printer size={22} color="var(--accent-primary)" />
              <div>
                <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                  Physical Scanner Bridge
                </h3>
                <p style={{ margin: '2px 0 0', fontSize: '0.78rem', color: 'var(--text-secondary)' }}>
                  Hardware WIA/TWAIN scanner trigger via NAPS2 CLI
                </p>
              </div>
            </div>

            <div
              style={{
                backgroundColor: 'var(--bg-tertiary)',
                padding: '14px',
                borderRadius: '8px',
                border: '1px solid var(--border-color)',
                display: 'flex',
                flexDirection: 'column',
                gap: '8px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <ShieldCheck size={16} color="#10b981" />
                <span style={{ fontSize: '0.84rem', fontWeight: 600, color: 'var(--text-primary)' }}>
                  Connected Scanner Detected
                </span>
              </div>
              <span style={{ fontSize: '0.76rem', color: 'var(--text-secondary)' }}>
                Default Office Document Scanner (WIA / USB Auto-Detect)
              </span>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
              <div>
                <label style={{ fontSize: '0.78rem', fontWeight: 600, color: 'var(--text-muted)' }}>FEEDER SOURCE</label>
                <select
                  style={{
                    width: '100%',
                    marginTop: '4px',
                    padding: '8px 12px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--bg-tertiary)',
                    color: 'var(--text-primary)',
                    border: '1px solid var(--border-color)',
                    fontSize: '0.84rem',
                  }}
                >
                  <option>ADF (Automatic Document Feeder) - Continuous</option>
                  <option>Flatbed Glass - Single Page</option>
                </select>
              </div>

              <div>
                <label style={{ fontSize: '0.78rem', fontWeight: 600, color: 'var(--text-muted)' }}>RESOLUTION (DPI)</label>
                <select
                  style={{
                    width: '100%',
                    marginTop: '4px',
                    padding: '8px 12px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--bg-tertiary)',
                    color: 'var(--text-primary)',
                    border: '1px solid var(--border-color)',
                    fontSize: '0.84rem',
                  }}
                >
                  <option>300 DPI (Recommended for OCR)</option>
                  <option>200 DPI (Fast / Draft)</option>
                  <option>600 DPI (Archival / High-Res)</option>
                </select>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '4px' }}>
                <input type="checkbox" defaultChecked id="deskew" />
                <label htmlFor="deskew" style={{ fontSize: '0.82rem', color: 'var(--text-primary)' }}>
                  Auto-deskew tilted pages & drop blank sheets
                </label>
              </div>
            </div>

            <button
              onClick={() => alert('Scanner feeder triggered! Scanned pages will stream into your active batch.')}
              style={{
                marginTop: 'auto',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                padding: '12px',
                borderRadius: '8px',
                border: 'none',
                backgroundColor: 'var(--accent-primary)',
                color: '#fff',
                fontWeight: 700,
                fontSize: '0.9rem',
                cursor: 'pointer',
              }}
            >
              <Zap size={16} />
              Scan Batch from Feeder
            </button>
          </div>

          {/* Right: Create Batch from Vault Documents */}
          <div
            style={{
              backgroundColor: 'var(--bg-secondary)',
              borderRadius: '12px',
              border: '1px solid var(--border-color)',
              padding: '24px',
              display: 'flex',
              flexDirection: 'column',
              gap: '18px',
            }}
          >
            <div>
              <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                Create Extraction Batch
              </h3>
              <p style={{ margin: '2px 0 0', fontSize: '0.78rem', color: 'var(--text-secondary)' }}>
                Queue hundreds of vault documents or uploaded files for batch processing
              </p>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
              <div>
                <label style={{ fontSize: '0.78rem', fontWeight: 600, color: 'var(--text-muted)' }}>BATCH TITLE</label>
                <input
                  placeholder="e.g. Q4 Vendor Invoices"
                  value={newBatchTitle}
                  onChange={(e) => setNewBatchTitle(e.target.value)}
                  style={{
                    width: '100%',
                    marginTop: '4px',
                    padding: '8px 12px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--bg-tertiary)',
                    color: 'var(--text-primary)',
                    border: '1px solid var(--border-color)',
                    fontSize: '0.84rem',
                    boxSizing: 'border-box',
                    outline: 'none',
                  }}
                />
              </div>

              <div>
                <label style={{ fontSize: '0.78rem', fontWeight: 600, color: 'var(--text-muted)' }}>TARGET SCHEMA</label>
                <select
                  value={newBatchSchemaId}
                  onChange={(e) => setNewBatchSchemaId(Number(e.target.value))}
                  style={{
                    width: '100%',
                    marginTop: '4px',
                    padding: '8px 12px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--bg-tertiary)',
                    color: 'var(--text-primary)',
                    border: '1px solid var(--border-color)',
                    fontSize: '0.84rem',
                  }}
                >
                  {schemas.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.name} ({s.columns.length} columns)
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label style={{ fontSize: '0.78rem', fontWeight: 600, color: 'var(--text-muted)' }}>
                  SELECT SOURCE DOCUMENTS ({documents?.length || 0} in Vault)
                </label>
                <div
                  style={{
                    maxHeight: '180px',
                    overflowY: 'auto',
                    backgroundColor: 'var(--bg-tertiary)',
                    borderRadius: '8px',
                    border: '1px solid var(--border-color)',
                    padding: '8px',
                    marginTop: '4px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '4px',
                  }}
                >
                  {documents?.slice(0, 30).map((d) => {
                    const isSelected = selectedDocIds.includes(d.id)
                    return (
                      <div
                        key={d.id}
                        onClick={() => {
                          setSelectedDocIds((prev) =>
                            isSelected ? prev.filter((id) => id !== d.id) : [...prev, d.id]
                          )
                        }}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: '8px',
                          padding: '6px 8px',
                          borderRadius: '6px',
                          backgroundColor: isSelected ? 'var(--accent-soft)' : 'transparent',
                          color: isSelected ? 'var(--accent-primary)' : 'var(--text-primary)',
                          fontSize: '0.8rem',
                          cursor: 'pointer',
                        }}
                      >
                        <input type="checkbox" checked={isSelected} readOnly />
                        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                          {d.title || d.original_file_name}
                        </span>
                      </div>
                    )
                  })}
                </div>
              </div>
            </div>

            <button
              onClick={handleCreateNewBatch}
              disabled={!newBatchTitle}
              style={{
                marginTop: 'auto',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                padding: '12px',
                borderRadius: '8px',
                border: 'none',
                backgroundColor: newBatchTitle ? '#10b981' : 'var(--bg-tertiary)',
                color: newBatchTitle ? '#fff' : 'var(--text-muted)',
                fontWeight: 700,
                fontSize: '0.9rem',
                cursor: newBatchTitle ? 'pointer' : 'not-allowed',
              }}
            >
              <Play size={16} />
              Start Batch Ingestion & Extraction
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
