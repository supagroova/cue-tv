import { useEffect, useState } from 'react';
import { Sheet } from './Sheets.jsx';
import { api } from './api.js';

export function AppleTVConnect({ open, onClose }) {
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [attempt, setAttempt] = useState(0);
  const [expired, setExpired] = useState(false);
  useEffect(() => {
    if (!open) return undefined;
    let live = true; let timer;
    setCode(''); setError(''); setExpired(false);
    api.appleTVCode().then((result) => {
      if (!live) return;
      setCode(result.code);
      timer = setTimeout(() => { if (live) { setCode(''); setExpired(true); } }, result.expires_in * 1000);
    }).catch((err) => { if (live) setError(err.message || 'Could not get a connection code.'); });
    return () => { live = false; clearTimeout(timer); };
  }, [open, attempt]);
  return <Sheet open={open} onClose={onClose} title="Connect Apple TV">
    <div className="apple-connect">
      <p>Open TV on your Apple TV and enter this code to connect your library and iPhone remote.</p>
      {code && <strong className="apple-connect-code" aria-label={`Connection code ${code.split('').join(' ')}`}>{code.slice(0, 3)} {code.slice(3)}</strong>}
      {!code && !error && !expired && <p role="status">Getting a code…</p>}
      {error && <p role="alert">{error}</p>}
      {expired && <p>This code has expired.</p>}
      {code && <small>The code lasts three minutes. Your TV's input stays as it is.</small>}
      {(error || expired) && <button className="pill glass" onClick={() => setAttempt((n) => n + 1)}>Get a New Code</button>}
    </div>
  </Sheet>;
}
