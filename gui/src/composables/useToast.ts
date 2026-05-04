export type ToastType = 'success' | 'info' | 'warning' | 'error';

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
  duration: number | false;
  createdAt: number;
}

interface ToastOptions {
  message: string;
  duration?: number | false;
}

const state = reactive<{ toasts: Toast[] }>({ toasts: [] });
let nextId = 0;

function add(type: ToastType, options: ToastOptions | string) {
  const opts: ToastOptions = typeof options === 'string' ? { message: options } : options;
  const duration = opts.duration === false ? false : (opts.duration ?? 30000);
  const toast: Toast = { id: nextId++, type, message: opts.message, duration, createdAt: Date.now() };
  state.toasts.push(toast);
  return toast.id;
}

function dismiss(id: number) {
  const idx = state.toasts.findIndex(t => t.id === id);
  if (idx !== -1) state.toasts.splice(idx, 1);
}

export function useToast() {
  return {
    toasts: computed(() => state.toasts),
    dismiss,
    success: (options: ToastOptions | string) => add('success', options),
    info: (options: ToastOptions | string) => add('info', options),
    warning: (options: ToastOptions | string) => add('warning', options),
    error: (options: ToastOptions | string) => add('error', options),
  };
}
