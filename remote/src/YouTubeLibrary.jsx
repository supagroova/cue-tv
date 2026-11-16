import { useEffect, useId, useRef, useState } from 'react';
import { api } from './api.js';
import { tick } from './haptics.js';
import { Gear } from './icons.jsx';

const LABELS = { search: 'Search', subscriptions: 'Subscriptions', playlists: 'Playlists', liked: 'Liked' };
const NOTICE = {
  expired: 'The code expired. Connect again for a new one.',
  denied: 'Google access wasn’t granted. You can connect again or keep searching.',
  reconnect: 'Your Google connection expired or was removed. Connect again on this phone.',
};

export function YouTubeLibrary({ active, view, onView, renderVideos, homeAction, searchLabel = 'Search', onHomeAccount, onManageFeeds }) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const settingsButton = useRef(null);
  const settingsId = useId();
  const [account, setAccount] = useState(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [detail, setDetail] = useState(null);
  const [list, setList] = useState({ status: 'idle', items: [], next: null });
  const [attempt, setAttempt] = useState(0);
  const version = useRef(0);
  const listRequest = useRef(0);
  const connection = account?.connection;
  const refresh = () => {
    const current = version.current;
    return api.youtubeAccount().then((result) => { if (current === version.current) { setAccount(result); setError(''); } })
      .catch((e) => { if (current === version.current && e.status !== 404) setError('Couldn’t check your YouTube connection.'); });
  };
  useEffect(() => {
    if (!active) return undefined;
    let live = true;
    const current = version.current;
    api.youtubeAccount().then((r) => { if (live && current === version.current) { setAccount(r); setError(''); } })
      .catch((e) => { if (live && current === version.current && e.status !== 404) setError('Couldn’t check your YouTube connection.'); });
    window.addEventListener('tv:resume', refresh);
    return () => { live = false; window.removeEventListener('tv:resume', refresh); };
  }, [active]);
  useEffect(() => {
    if (!active || account?.state !== 'pending') return undefined;
    let live = true;
    const current = version.current;
    const timer = setTimeout(() => {
      api.youtubeLinkStatus().then((r) => { if (live && current === version.current) { setAccount(r); setError(''); } })
        .catch((e) => { if (live && current === version.current) { setError(e.message || 'Couldn’t reach Google. Try again.'); setAccount((a) => ({ ...a })); } });
    }, Math.max(5, account.link?.interval || 5) * 1000);
    return () => { live = false; clearTimeout(timer); };
  }, [account, active]);
  useEffect(() => {
    listRequest.current += 1; setDetail(null); setList({ status: 'idle', items: [], next: null });
    onView('search');
  }, [connection]);
  useEffect(() => { setDetail(null); }, [view]);
  useEffect(() => { if (!active) setSettingsOpen(false); }, [active]);
  const kind = detail?.kind || view;
  const itemId = detail?.id || '';
  useEffect(() => {
    if (view === 'search' || !account?.connected) return undefined;
    let live = true;
    const request = ++listRequest.current;
    setList({ status: 'loading', items: [], next: null });
    api.youtubeLibrary(kind, itemId).then((r) => { if (live && request === listRequest.current) setList({ status: 'ready', ...r }); })
      .catch((e) => { if (live && request === listRequest.current) { setList({ status: 'error', items: [], next: null, error: e.message }); if (e.status === 409) refresh(); } });
    return () => { live = false; };
  }, [view, kind, itemId, connection, attempt, account?.connected]);
  const action = async (operation) => {
    if (busy) return;
    tick(); version.current += 1; setBusy(true); setError('');
    try { setAccount(await operation()); }
    catch (e) { setError(e.message || 'Couldn’t update your YouTube connection.'); }
    finally { setBusy(false); }
  };
  const disconnect = () => {
    listRequest.current += 1; setList({ status: 'idle', items: [], next: null }); setDetail(null); onView('search');
    action(api.youtubeDisconnect);
  };
  const more = async () => {
    if (!list.next || list.more) return;
    const request = listRequest.current;
    setList((r) => ({ ...r, more: true, moreError: '' }));
    try {
      const result = await api.youtubeLibrary(kind, itemId, list.next);
      if (request === listRequest.current) setList((r) => ({ ...result, status: 'ready', items: [...r.items, ...result.items.filter((item) => !r.items.some((old) => old.id === item.id))], more: false }));
    } catch (e) { if (request === listRequest.current) setList((r) => ({ ...r, more: false, moreError: e.message })); }
  };
  const closeSettings = (event) => {
    if (event.key !== 'Escape') return;
    event.preventDefault(); event.stopPropagation(); setSettingsOpen(false);
    settingsButton.current?.focus({ preventScroll: true });
  };
  return <div className="youtube-library">
    <div className="youtube-utilities">
      {homeAction}
      <button ref={settingsButton} className="youtube-utility youtube-settings-toggle" aria-label="YouTube settings" aria-expanded={settingsOpen} aria-controls={settingsId}
        onKeyDown={closeSettings} onClick={() => { tick(); setSettingsOpen((value) => !value); }}><Gear size={16} />Settings</button>
    </div>
    {settingsOpen && <section id={settingsId} className="youtube-account youtube-settings" aria-label="YouTube account" onKeyDown={closeSettings}>
      <h3>YouTube settings</h3>
      {!account ? <p role="status">{error || 'Checking your connection…'}</p> : account.connected ? <>
        <div className="youtube-account-row"><div>{account.account?.thumbnail && <img src={account.account.thumbnail} alt="" />}<span><b>{account.account?.title || 'YouTube account'}</b><small>Connected on this phone</small></span></div>
          <button disabled={busy} onClick={disconnect}>Disconnect this phone</button></div>
        <p>Videos play with the YouTube account selected on your TV.</p>
      </> : account.state === 'pending' && account.link ? <>
        <b>Connect your YouTube account</b><p>Open Google, enter this code and choose the account you want on this phone.</p>
        <code className="youtube-user-code">{account.link.user_code}</code>
        <a className="stremio-connect-button" href={account.link.verification_url} target="_blank" rel="noreferrer">Continue to Google</a>
        <button className="youtube-cancel" disabled={busy} onClick={() => action(api.youtubeCancel)}>Cancel connection</button>
        <p>Return here after approving. This code expires in {Math.max(1, Math.ceil(account.link.expires_in / 60))} minutes.</p>
      </> : <>
        <b>Your YouTube library</b><p>Connect your account for subscriptions, playlists and liked videos. Each phone keeps its own connection.</p>
        {account.configured ? <button className="stremio-connect-button" disabled={busy} onClick={() => action(api.youtubeConnect)}>{busy ? 'Connecting…' : 'Connect YouTube'}</button>
          : <p>Account connections aren’t ready on this TV yet. Search still works.</p>}
        {NOTICE[account.state] && <p role="status">{NOTICE[account.state]}</p>}
      </>}
      {error && account && <p role="status">{error}</p>}
      {error && <button className="stremio-connect-button" onClick={refresh}>Try again</button>}
      <p className="youtube-settings-note">Google doesn’t share Watch History or Watch Later with this app.</p>
      {window.couchNative?.canInlineYouTube && <>
        <p className="youtube-settings-note">Each Home feed has its own sign-in on this phone. Subscriptions, playlists and liked videos use the library account above.</p>
        <div className="youtube-settings-links">
          <button className="youtube-feed-setting" onClick={() => { setSettingsOpen(false); onHomeAccount?.(); }}>Sign in or change feed account</button>
          {window.couchNative?.canManageYouTubeFeeds && <button className="youtube-feed-setting" onClick={() => { setSettingsOpen(false); onManageFeeds?.(); }}>Manage YouTube feeds</button>}
        </div>
      </>}
      <div className="youtube-settings-links">
        {account?.connected && <a href="https://myaccount.google.com/connections" target="_blank" rel="noreferrer">Manage Google access</a>}
        <a href="/youtube/privacy.html" target="_blank" rel="noreferrer">YouTube privacy</a>
      </div>
    </section>}
    {account?.connected && <>
      <div className="youtube-library-tabs" role="group" aria-label="Your YouTube library">
        {Object.entries(LABELS).map(([id, label]) => <button key={id} aria-pressed={view === id} onClick={() => { tick(); setSettingsOpen(false); setDetail(null); onView(id); }}>{id === 'search' ? searchLabel : label}</button>)}
      </div>
      {view !== 'search' && <section className="youtube-personal-list" aria-label={detail?.title || LABELS[view]} aria-busy={list.status === 'loading'}>
        {detail && <button className="youtube-list-back" onClick={() => { tick(); setDetail(null); }}>‹ {LABELS[view]}</button>}
        <h3>{detail?.title || LABELS[view]}</h3>
        {list.status === 'loading' && <p className="stremio-search-status" role="status">Loading…</p>}
        {list.status === 'error' && <div className="stremio-search-status" role="status"><p>{list.error || 'Couldn’t load this list.'}</p><button className="stremio-connect-button" onClick={() => setAttempt((a) => a + 1)}>Try again</button></div>}
        {list.status === 'ready' && (list.items.length ? list.kind === 'videos' ? renderVideos(list.items) : <div className="youtube-collections">
          {list.items.map((item) => <button key={item.id} className="youtube-collection" onClick={() => { tick(); setDetail({ kind: view === 'subscriptions' ? 'channel' : 'playlist', id: item.id, title: item.title }); }}>
            {item.thumbnail ? <img src={item.thumbnail} alt="" loading="lazy" /> : <span aria-hidden="true">{item.title.slice(0, 1)}</span>}
            <b>{item.title}</b>{view === 'playlists' && <small>{item.count} videos</small>}
          </button>)}
        </div> : <p className="stremio-search-status">Nothing in this list yet.</p>)}
        {list.moreError && <p className="stremio-search-status" role="status">{list.moreError}</p>}
        {list.next && list.status === 'ready' && <button className="stremio-connect-button youtube-load-more" disabled={list.more} onClick={more}>{list.more ? 'Loading…' : 'Load more'}</button>}
      </section>}
    </>}
  </div>;
}
