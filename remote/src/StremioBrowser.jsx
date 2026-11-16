import { useEffect, useRef, useState } from 'react';
import { api } from './api.js';
import { tick } from './haptics.js';
import { Close, Search } from './icons.jsx';
import { PlatformSelector } from './PlatformSelector.jsx';
import { StremioDetail } from './StremioDetail.jsx';

function Titles({ title, items, onOpen }) {
  if (!items.length) return null;
  return <section className="stremio-shelf">
    <h3>{title}</h3>
    <div className="shelf">
      {items.map((item) => <button key={`${item.type}:${item.id}`} className="poster stremio-title" onClick={() => { tick(); onOpen(item); }}>
        <span className="poster-art">
          {item.poster ? <img src={item.poster} alt="" loading="lazy" /> : <span>{item.title}</span>}
          {typeof item.progress === 'number' && item.progress > 0 && <span className="stremio-progress"><i style={{ width: `${Math.round(Math.min(item.progress, 1) * 100)}%` }} /></span>}
        </span>
        <b>{item.title}</b><small>{item.year || (item.type === 'series' ? 'Series' : 'Film')}</small>
      </button>)}
    </div>
  </section>;
}

export function StremioBrowser({ onOpen, onPlaySource, onLaunch, active, platform, platformBusy, platformDisabled, onPlatform }) {
  const [selected, setSelected] = useState(null);
  const [catalogue, setCatalogue] = useState(null);
  const [library, setLibrary] = useState(null);
  const [link, setLink] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [query, setQuery] = useState('');
  const [search, setSearch] = useState({ status: 'idle', items: [] });
  const [searchAttempt, setSearchAttempt] = useState(0);
  const searchInput = useRef(null);
  const searching = Boolean(query.trim());

  useEffect(() => {
    const term = query.trim();
    if (!term) { setSearch({ status: 'idle', items: [] }); return undefined; }
    let live = true;
    setSearch({ status: 'loading', items: [] });
    const timer = setTimeout(() => {
      api.stremioSearch(term).then((result) => {
        if (live) setSearch({ status: 'ready', items: result.items || [] });
      }).catch(() => {
        if (live) setSearch({ status: 'error', items: [] });
      });
    }, 300);
    return () => { live = false; clearTimeout(timer); };
  }, [query, searchAttempt]);

  const refreshLibrary = () => api.stremioLibrary().then(setLibrary).catch(() => setError('Could not load your Stremio history.'));
  useEffect(() => {
    api.stremioDiscover().then(setCatalogue).catch(() => setCatalogue(false));
    refreshLibrary();
    const refresh = () => refreshLibrary();
    window.addEventListener('tv:resume', refresh);
    return () => window.removeEventListener('tv:resume', refresh);
  }, []);
  useEffect(() => { if (active) refreshLibrary(); }, [active]);
  useEffect(() => {
    if (!link) return undefined;
    let live = true;
    const interval = setInterval(() => {
      api.stremioLinkStatus().then((r) => {
        if (live && r.connected) { setLink(''); setError(''); refreshLibrary(); api.stremioDiscover().then(setCatalogue).catch(() => {}); }
      }).catch(() => {});
    }, 3000);
    const expiry = setTimeout(() => { if (live) { setLink(''); setError('The link expired. Choose Connect Stremio to try again.'); } }, 5 * 60 * 1000);
    return () => { live = false; clearInterval(interval); clearTimeout(expiry); };
  }, [link]);

  const connect = async () => {
    setBusy(true); setError('');
    try { const r = await api.stremioLink(); setLink(r.link); }
    catch (e) { setError(e.message || 'Could not connect to Stremio.'); }
    finally { setBusy(false); }
  };
  const disconnect = async () => {
    setBusy(true); setError('');
    try { await api.stremioUnlink(); setLibrary({ connected: false, items: [] }); setLink(''); }
    catch (e) { setError(e.message || 'Could not remove the connection.'); }
    finally { setBusy(false); }
  };
  const items = library?.items || [];
  if (selected) return <StremioDetail key={`${selected.type}:${selected.id}`} item={selected} onBack={() => setSelected(null)} onPlay={onPlaySource} onOpenTV={onOpen} platform={platform} platformBusy={platformBusy} platformDisabled={platformDisabled} onPlatform={onPlatform} />;
  const selectTitle = (item) => { searchInput.current?.blur(); setSelected(item); };
  return <div className="stremio-browser">
    <section className="stremio-intro">
      <div className="stremio-destination"><div className="stremio-kicker">YOUR TV</div><PlatformSelector platform={platform} busy={platformBusy} disabled={platformDisabled} onSelect={onPlatform} /></div>
      <div className="stremio-intro-row"><div><h2>Watch Now</h2><p>Find something to watch on Stremio.</p></div>
        <button className="stremio-open" onClick={() => { tick(); onLaunch({ id: 'stremio', name: 'Stremio' }); }} aria-label="Open Stremio on the TV">▶</button>
      </div>
      <form className="stremio-search" role="search" onSubmit={(event) => { event.preventDefault(); searchInput.current?.blur(); }}>
        <Search size={20} />
        <input ref={searchInput} type="search" name="watch-search" aria-label="Search films and TV shows" placeholder="Search films and TV shows" value={query} maxLength={100}
          autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false} enterKeyHint="search"
          onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === 'Escape') setQuery(''); }} />
        {query && <button type="button" aria-label="Clear search" onClick={() => { setQuery(''); searchInput.current?.focus(); }}><Close size={16} /></button>}
      </form>
      {!searching && !library?.connected && <div className="stremio-connect">
        <div><b>Bring your watch history</b><p>Connect your Stremio account to see what you’ve been watching.</p></div>
        {link ? <a className="stremio-connect-button" href={link} target="_blank" rel="noreferrer">Continue to Stremio</a>
          : <button className="stremio-connect-button" disabled={busy} onClick={connect}>{busy ? 'Connecting…' : 'Connect Stremio'}</button>}
        {link && <p>Sign in on Stremio’s own page, then return here.</p>}
      </div>}
      {!searching && library?.connected && <div className="stremio-connected"><span>Stremio connected</span><button disabled={busy} onClick={disconnect}>Remove connection</button></div>}
      {!searching && error && <p className="stremio-error" role="status">{error}</p>}
    </section>
    {searching ? <div aria-label="Search results" aria-busy={search.status === 'loading'}>
      {search.status === 'loading' && <p className="stremio-search-status" role="status">Searching…</p>}
      {search.status === 'error' && <div className="stremio-search-status" role="status"><p>Couldn’t search right now. Please try again.</p><button className="stremio-connect-button" onClick={() => setSearchAttempt((attempt) => attempt + 1)}>Try again</button></div>}
      {search.status === 'ready' && (search.items.length ? <>
        <Titles title="TV Shows" items={search.items.filter((item) => item.type === 'series')} onOpen={selectTitle} />
        <Titles title="Films" items={search.items.filter((item) => item.type === 'movie')} onOpen={selectTitle} />
      </> : <p className="stremio-search-status" role="status">No titles found for “{query.trim()}”.</p>)}
    </div> : <>
    <Titles title="Continue Watching" items={items.filter((item) => item.inProgress)} onOpen={selectTitle} />
    <Titles title="Recently Watched" items={items.filter((item) => !item.inProgress)} onOpen={selectTitle} />
    <Titles title="Trending Series" items={catalogue?.trendingSeries || []} onOpen={selectTitle} />
    <Titles title="Trending Films" items={catalogue?.trendingMovies || []} onOpen={selectTitle} />
    <Titles title="Popular Series" items={catalogue?.series || []} onOpen={selectTitle} />
    <Titles title="Popular Films" items={catalogue?.movies || []} onOpen={selectTitle} />
    {catalogue === false && <p className="empty">The Stremio catalogue is unavailable right now.</p>}
    </>}
  </div>;
}
