import { useEffect, useRef, useState } from 'react';
import { api } from './api.js';
import { tick } from './haptics.js';
import { Close, Home, Search } from './icons.jsx';
import { PlatformSelector } from './PlatformSelector.jsx';
import { YouTubeLibrary } from './YouTubeLibrary.jsx';
import { YouTubeHomeFeed } from './YouTubeHomeFeed.jsx';

export function YouTubeBrowser({ onOpen, onLaunch, active, platform, platformBusy, platformDisabled, onPlatform }) {
  const [query, setQuery] = useState('');
  const [search, setSearch] = useState({ status: 'idle', items: [] });
  const [attempt, setAttempt] = useState(0);
  const [opening, setOpening] = useState('');
  const [view, setView] = useState('search');
  const input = useRef(null);
  useEffect(() => {
    const term = query.trim();
    if (!term) { setSearch({ status: 'idle', items: [] }); return undefined; }
    let live = true;
    setSearch({ status: 'loading', items: [] });
    const timer = setTimeout(() => {
      api.youtubeSearch(term).then((result) => {
        if (live) setSearch({ status: 'ready', items: result.items || [] });
      }).catch(() => { if (live) setSearch({ status: 'error', items: [] }); });
    }, 350);
    return () => { live = false; clearTimeout(timer); };
  }, [query, attempt]);

  const open = async (item) => {
    if (opening) return;
    tick(); input.current?.blur(); setOpening(item.id);
    try { await onOpen(item); } finally { setOpening(''); }
  };
  const videos = (items) => <div className="youtube-results">
    {items.map((item) => <button key={item.id} className="youtube-video" disabled={Boolean(opening)} aria-label={`Watch ${item.title}`} onClick={() => open(item)}>
      <span className="youtube-thumbnail"><img src={item.thumbnail} alt="" loading="lazy" />{item.duration && <small>{item.duration}</small>}</span>
      <span className="youtube-video-info"><b>{item.title}</b><small>{opening === item.id ? 'Opening on TV…' : item.channel}</small></span>
    </button>)}
  </div>;
  return <div className="youtube-browser">
    <section className="stremio-intro">
      <div className="stremio-destination"><div className="stremio-kicker">YOUR TV</div><PlatformSelector platform={platform} busy={platformBusy} disabled={platformDisabled} onSelect={onPlatform} /></div>
      <div className="stremio-intro-row"><div><h2>YouTube</h2><p>Find a video. Watch it on the big screen.</p></div>
        <button className="stremio-open" onClick={() => { tick(); onLaunch({ id: 'youtube', name: 'YouTube' }); }} aria-label="Open YouTube on the TV">▶</button>
      </div>
      <form className="stremio-search" role="search" onSubmit={(event) => { event.preventDefault(); input.current?.blur(); }}>
        <Search size={20} />
        <input ref={input} type="search" name="youtube-search" aria-label="Search YouTube" placeholder="Search YouTube" value={query} maxLength={100}
          autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false} enterKeyHint="search"
          onChange={(event) => { setQuery(event.target.value); setView('search'); }} onKeyDown={(event) => { if (event.key === 'Escape') setQuery(''); }} />
        {query && <button type="button" aria-label="Clear YouTube search" onClick={() => { setQuery(''); if (window.couchNative?.canInlineYouTube) input.current?.blur(); else input.current?.focus(); }}><Close size={16} /></button>}
      </form>
    </section>
    <YouTubeLibrary active={active} view={view} onView={setView} renderVideos={videos} searchLabel={!query.trim() && window.couchNative?.canInlineYouTube ? 'Home' : 'Search'}
      onHomeAccount={() => { tick(); input.current?.blur(); setQuery(''); setView('search'); window.couchNative?.openYouTubeAccount?.(); }}
      onManageFeeds={() => { tick(); input.current?.blur(); setQuery(''); setView('search'); window.couchNative?.manageYouTubeFeeds?.(); }} homeAction={window.couchNative?.canBrowseYouTube
      ? <button className="youtube-utility" aria-label="Open YouTube Home" onClick={() => { tick(); input.current?.blur(); window.couchNative.openYouTubeHome(); }}><Home size={16} />YouTube Home</button>
      : <a className="youtube-utility" aria-label="Open YouTube Home" href="https://www.youtube.com/" target="_blank" rel="noreferrer"><Home size={16} />YouTube Home</a>} />
    {view === 'search' && <div aria-label="YouTube results" aria-busy={search.status === 'loading'}>
      {search.status === 'idle' && (window.couchNative?.canInlineYouTube ? <YouTubeHomeFeed active={active} /> : <div className="youtube-empty"><Search size={32} /><h3>What would you like to watch?</h3><p>Search for videos, channels or anything you’re into.</p></div>)}
      {search.status === 'loading' && <p className="stremio-search-status" role="status">Searching YouTube…</p>}
      {search.status === 'error' && <div className="stremio-search-status" role="status"><p>Couldn’t search YouTube right now.</p><button className="stremio-connect-button" onClick={() => setAttempt((a) => a + 1)}>Try again</button><p>You can also open YouTube on the TV.</p></div>}
      {search.status === 'ready' && (search.items.length ? videos(search.items) : <p className="stremio-search-status" role="status">No videos found for “{query.trim()}”.</p>)}
    </div>}
  </div>;
}
