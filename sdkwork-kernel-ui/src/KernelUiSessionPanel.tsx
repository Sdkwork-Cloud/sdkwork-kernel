import { useState } from 'react';
import { isBlank, trim } from '@sdkwork/utils';
import { translateKernelUi } from '@sdkwork/kernel-ui-commons';
import {
  clearBrowserKernelUiAuthSession,
  persistBrowserKernelUiAuthSession
} from '@sdkwork/kernel-ui-services';
import type { KernelUiAuthSession } from '@sdkwork/kernel-ui-types';

interface KernelUiSessionPanelProps {
  onSessionSaved: (session: KernelUiAuthSession) => void;
}

export function KernelUiSessionPanel({ onSessionSaved }: KernelUiSessionPanelProps) {
  const [accessToken, setAccessToken] = useState('');
  const [tenantId, setTenantId] = useState('');
  const [userId, setUserId] = useState('');

  const handleSave = () => {
    const session: KernelUiAuthSession = {
      accessToken: trim(accessToken),
      tenantId: isBlank(tenantId) ? undefined : trim(tenantId),
      userId: isBlank(userId) ? undefined : trim(userId)
    };
    if (isBlank(session.accessToken)) {
      return;
    }
    persistBrowserKernelUiAuthSession(session);
    onSessionSaved(session);
  };

  const handleClear = () => {
    clearBrowserKernelUiAuthSession();
    setAccessToken('');
    setTenantId('');
    setUserId('');
  };

  return (
    <main className="kernel-ui-shell kernel-ui-auth">
      <section className="kernel-panel kernel-ui-auth__panel">
        <p className="kernel-panel__eyebrow">{translateKernelUi('auth.title')}</p>
        <h2>{translateKernelUi('app.title')}</h2>
        <p className="kernel-ui-auth__description">{translateKernelUi('auth.description')}</p>
        <label className="kernel-ui-auth__field">
          <span>{translateKernelUi('auth.accessToken')}</span>
          <input
            type="password"
            value={accessToken}
            onChange={(event) => setAccessToken(event.target.value)}
            autoComplete="off"
          />
        </label>
        <label className="kernel-ui-auth__field">
          <span>{translateKernelUi('auth.tenantId')}</span>
          <input value={tenantId} onChange={(event) => setTenantId(event.target.value)} />
        </label>
        <label className="kernel-ui-auth__field">
          <span>{translateKernelUi('auth.userId')}</span>
          <input value={userId} onChange={(event) => setUserId(event.target.value)} />
        </label>
        <div className="kernel-ui-auth__actions">
          <button type="button" className="kernel-ui-auth__button" onClick={handleSave}>
            {translateKernelUi('auth.save')}
          </button>
          <button
            type="button"
            className="kernel-ui-auth__button kernel-ui-auth__button--secondary"
            onClick={handleClear}
          >
            {translateKernelUi('auth.clear')}
          </button>
        </div>
      </section>
    </main>
  );
}
