import { create } from 'zustand'

export type ModalTone = 'info' | 'error' | 'success' | 'warning' | 'danger'

export interface ConfirmModalOptions {
  title?: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
  type?: ModalTone
}

export interface AlertModalOptions {
  title?: string
  message: string
  type?: ModalTone
}

export interface PromptModalOptions {
  title?: string
  message: string
  placeholder?: string
  defaultValue?: string
  confirmText?: string
  cancelText?: string
}

interface ModalState {
  isOpen: boolean
  modalType: 'confirm' | 'alert' | 'prompt' | null
  title: string
  message: string
  confirmText: string
  cancelText: string
  danger: boolean
  alertType: ModalTone
  promptPlaceholder: string
  promptValue: string
  resolveConfirm: ((val: boolean) => void) | null
  resolveAlert: (() => void) | null
  resolvePrompt: ((val: string | null) => void) | null

  confirm: (options: string | ConfirmModalOptions) => Promise<boolean>
  alert: (options: string | AlertModalOptions) => Promise<void>
  prompt: (options: string | PromptModalOptions) => Promise<string | null>
  close: () => void
  submitPrompt: (val: string) => void
}

export const useModalStore = create<ModalState>((set, get) => ({
  isOpen: false,
  modalType: null,
  title: '',
  message: '',
  confirmText: 'Confirm',
  cancelText: 'Cancel',
  danger: false,
  alertType: 'info',
  promptPlaceholder: '',
  promptValue: '',
  resolveConfirm: null,
  resolveAlert: null,
  resolvePrompt: null,

  confirm: (options) => {
    return new Promise<boolean>((resolve) => {
      const opts: ConfirmModalOptions =
        typeof options === 'string' ? { message: options } : options
      const isDangerous =
        opts.danger ||
        opts.type === 'danger' ||
        opts.message.toLowerCase().includes('delete') ||
        opts.message.toLowerCase().includes('trash')
      set({
        isOpen: true,
        modalType: 'confirm',
        title: opts.title || (isDangerous ? 'Confirm Action' : 'Confirm'),
        message: opts.message,
        confirmText: opts.confirmText || (isDangerous ? 'Delete' : 'Confirm'),
        cancelText: opts.cancelText || 'Cancel',
        danger: isDangerous,
        alertType: opts.type || (isDangerous ? 'danger' : 'info'),
        resolveConfirm: resolve,
      })
    })
  },

  alert: (options) => {
    return new Promise<void>((resolve) => {
      const opts: AlertModalOptions =
        typeof options === 'string' ? { message: options } : options
      const isErr =
        opts.type === 'error' ||
        opts.type === 'danger' ||
        opts.message.toLowerCase().includes('fail') ||
        opts.message.toLowerCase().includes('error')
      set({
        isOpen: true,
        modalType: 'alert',
        title: opts.title || (isErr ? 'Error' : 'Notice'),
        message: opts.message,
        confirmText: 'OK',
        cancelText: '',
        danger: isErr,
        alertType: opts.type || (isErr ? 'error' : 'info'),
        resolveAlert: resolve,
      })
    })
  },

  prompt: (options) => {
    return new Promise<string | null>((resolve) => {
      const opts: PromptModalOptions =
        typeof options === 'string' ? { message: options } : options
      set({
        isOpen: true,
        modalType: 'prompt',
        title: opts.title || 'Input Required',
        message: opts.message,
        confirmText: opts.confirmText || 'Save',
        cancelText: 'Cancel',
        promptPlaceholder: opts.placeholder || '',
        promptValue: opts.defaultValue || '',
        danger: false,
        resolvePrompt: resolve,
      })
    })
  },

  close: () => {
    const { resolveConfirm, resolveAlert, resolvePrompt } = get()
    if (resolveConfirm) resolveConfirm(false)
    if (resolveAlert) resolveAlert()
    if (resolvePrompt) resolvePrompt(null)
    set({
      isOpen: false,
      modalType: null,
      resolveConfirm: null,
      resolveAlert: null,
      resolvePrompt: null,
    })
  },

  submitPrompt: (val: string) => {
    const { resolvePrompt } = get()
    if (resolvePrompt) resolvePrompt(val)
    set({
      isOpen: false,
      modalType: null,
      resolvePrompt: null,
    })
  },
}))

// Standalone functions callable anywhere without React hooks
export const confirmModal = (options: string | ConfirmModalOptions) =>
  useModalStore.getState().confirm(options)

export const alertModal = (options: string | AlertModalOptions) =>
  useModalStore.getState().alert(options)

export const promptModal = (options: string | PromptModalOptions) =>
  useModalStore.getState().prompt(options)
