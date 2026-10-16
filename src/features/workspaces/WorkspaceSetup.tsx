import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Section } from '../../components/content/Section';
import { Button } from '../../components/ui/Button';
import { FeedbackMessage } from '../../components/ui/FeedbackMessage';
import '../../views/WorkspacesView.css';

export function WorkspaceSetup({
  adding,
  error,
  onAdd,
  onSkip,
  focusGeneration = 0,
  recoveryAction,
}: {
  adding: boolean;
  error: Error | null;
  onAdd: () => unknown;
  onSkip: () => unknown;
  focusGeneration?: number;
  recoveryAction?: ReactNode;
}) {
  const pending = useRef(false);
  const invoker = useRef<HTMLElement | null>(null);
  const invokerGeneration = useRef(0);
  const [busy, setBusy] = useState(false);
  useLayoutEffect(() => {
    if (!busy && !adding && invoker.current) {
      if (
        invoker.current.isConnected &&
        invokerGeneration.current === focusGeneration
      )
        invoker.current.focus();
      invoker.current = null;
    }
  }, [busy, adding, focusGeneration]);
  async function choose(action: () => unknown) {
    if (pending.current || adding) return;
    pending.current = true;
    invoker.current = document.activeElement as HTMLElement | null;
    invokerGeneration.current = focusGeneration;
    setBusy(true);
    try {
      await action();
    } catch {
      /* The controller retains the actionable error. */
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  return (
    <div className="workspace-setup">
      <Section title="Choose workspace folders">
        <p>Workspaces are optional. Adding a folder does not scan it.</p>
        <div className="workspace-actions">
          <Button
            disabled={adding || busy}
            onClick={() => {
              void choose(onAdd);
            }}
          >
            Add workspace
          </Button>
          <Button
            disabled={adding || busy}
            onClick={() => {
              void choose(onSkip);
            }}
          >
            Skip for now
          </Button>
        </div>
        {error && (
          <FeedbackMessage
            tone="error"
            title="Setup needs attention"
            action={recoveryAction}
          >
            {error.message}
          </FeedbackMessage>
        )}
      </Section>
    </div>
  );
}
