// Native dialogs provide inert backgrounds; Tab wraps within the active dialog. Each nested
// dialog remembers its own opener; removed rows fall back to a visible control.
const openers = new WeakMap<HTMLDialogElement, HTMLElement | null>()
const stack: HTMLDialogElement[] = []
function trapTab(event: KeyboardEvent) {
  if (event.key !== 'Tab') return
  const dialog = event.currentTarget as HTMLDialogElement
  const controls = [...dialog.querySelectorAll<HTMLElement>('button, a[href], input, select, textarea, [tabindex]')]
    .filter(element => !element.matches(':disabled') && element.tabIndex >= 0 && element.getClientRects().length > 0)
  const first = controls[0], last = controls.at(-1)
  if (!first) { event.preventDefault(); dialog.focus(); return }
  if (event.shiftKey && (document.activeElement === first || !dialog.contains(document.activeElement))) {
    event.preventDefault(); last?.focus()
  } else if (!event.shiftKey && (document.activeElement === last || !dialog.contains(document.activeElement))) {
    event.preventDefault(); first.focus()
  }
}
export function showDialog(dialog?: HTMLDialogElement) {
  if (!dialog || dialog.open) return
  openers.set(dialog, document.activeElement instanceof HTMLElement ? document.activeElement : null)
  stack.push(dialog)
  dialog.addEventListener('keydown',trapTab)
  dialog.showModal()
}
export function closeDialog(dialog?: HTMLDialogElement) {
  if (!dialog?.open) return
  const opener = openers.get(dialog)
  dialog.removeEventListener('keydown',trapTab)
  dialog.close()
  const index = stack.indexOf(dialog)
  if (index >= 0) stack.splice(index,1)
  queueMicrotask(() => {
    const top = stack.findLast(item => item.isConnected && item.open)
    if (opener?.isConnected && !opener.matches(':disabled') && (!top || top.contains(opener))) opener.focus({preventScroll:true})
    else (top?.querySelector<HTMLElement>('button:not(:disabled), input:not(:disabled), [tabindex="0"]') ?? document.querySelector<HTMLElement>('[data-focus-fallback]'))?.focus({preventScroll:true})
  })
}
