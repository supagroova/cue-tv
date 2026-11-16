import { useEffect, useRef, useState } from 'react';
import { api } from './api.js';
import { tick } from './haptics.js';
import { PlatformSelector } from './PlatformSelector.jsx';

export function StremioDetail({ item, onBack, onPlay, onOpenTV, platform, platformBusy, platformDisabled, onPlatform }) {
  const [detail, setDetail] = useState({ status: 'loading' });
  const [season, setSeason] = useState(null);
  const [episode, setEpisode] = useState(null);
  const [sources, setSources] = useState({ status: 'idle', items: [] });
  const [attempt, setAttempt] = useState(0);
  const [sourceAttempt, setSourceAttempt] = useState(0);
  const [filter, setFilter] = useState('');
  const [opening, setOpening] = useState('');
  const [playError, setPlayError] = useState('');
  const heading = useRef(null);
  const movie = item.type === 'movie';
  useEffect(() => {
    let live = true;
    setDetail({ status: 'loading' });
    api.stremioDetail(item).then((value) => {
      if (!live) return;
      setDetail({ status: 'ready', ...value });
      const seasons = [...new Set((value.videos || []).map((video) => video.season))].sort((a, b) => a - b);
      setSeason(seasons.find((value) => value > 0) ?? seasons[0] ?? null);
      if (movie) setEpisode({ id: item.id, title: value.title });
    }).catch((error) => { if (live) setDetail({ status: 'error', error: error.message }); });
    return () => { live = false; };
  }, [item.type, item.id, attempt]);
  useEffect(() => {
    if (!episode || detail.status !== 'ready') return undefined;
    let live = true;
    setSources({ status: 'loading', items: [] }); setFilter(''); setPlayError('');
    api.stremioSources(item, episode.id).then((value) => { if (live) setSources({ status: 'ready', ...value }); })
      .catch((error) => { if (live) setSources({ status: 'error', items: [], error: error.message }); });
    return () => { live = false; };
  }, [episode?.id, detail.status, sourceAttempt]);
  useEffect(() => { heading.current?.focus({ preventScroll: true }); }, [episode?.id]);
  const back = () => {
    if (opening) return;
    tick();
    if (episode && !movie) { setEpisode(null); setPlayError(''); } else onBack();
  };
  const play = async (source) => {
    if (opening || !source.playable) return;
    tick(); setOpening(source.id); setPlayError('');
    try { await onPlay(source); }
    catch (error) { setPlayError(error.message || 'Couldn’t open that source. Choose another or try again.'); }
    finally { setOpening(''); }
  };
  const future = (video) => Number.isFinite(Date.parse(video.released)) && Date.parse(video.released) > Date.now();
  const episodes = (detail.videos || []).filter((video) => video.season === season);
  const filtered = (sources.items || []).filter((source) => `${source.provider} ${source.name} ${source.description}`.toLowerCase().includes(filter.trim().toLowerCase()));
  const title = detail.title || item.title;
  return <div className="stremio-detail">
    <div className="stremio-detail-toolbar">
      <button className="stremio-back" disabled={Boolean(opening)} onClick={back}>‹ {episode && !movie ? 'Episodes' : 'Watch Now'}</button>
      <PlatformSelector platform={platform} busy={platformBusy} disabled={platformDisabled || Boolean(opening)} onSelect={onPlatform} />
    </div>
    <header className="stremio-detail-heading">
      {(detail.background || item.background || detail.poster || item.poster) && <img className="stremio-detail-art" src={detail.background || item.background || detail.poster || item.poster} alt="" />}
      <div><h2 tabIndex={-1} ref={heading}>{title}</h2>
        <p className="stremio-detail-meta">{[detail.year || item.year, detail.runtime, movie ? 'Film' : 'Series'].filter(Boolean).join(' · ')}</p>
        {episode && !movie ? <h3 className="stremio-selected-episode">S{episode.season} E{episode.episode} · {episode.title}</h3>
          : detail.description && <details className="stremio-description"><summary>About this {movie ? 'film' : 'show'}</summary><p>{detail.description}</p></details>}
      </div>
    </header>
    {detail.status === 'loading' && <p className="stremio-search-status" role="status">Loading {movie ? 'film details' : 'episodes'}…</p>}
    {detail.status === 'error' && <div className="stremio-search-status" role="status"><p>{detail.error}</p><button className="stremio-connect-button" onClick={() => setAttempt((value) => value + 1)}>Try again</button></div>}
    {detail.status === 'ready' && !episode && !movie && <section className="stremio-episodes" aria-label="Episodes">
      <div className="stremio-section-heading"><h3>Episodes</h3><label>Season<select aria-label="Season" value={season ?? ''} onChange={(event) => { tick(); setSeason(Number(event.target.value)); }}>
        {[...new Set((detail.videos || []).map((video) => video.season))].sort((a, b) => a - b).map((value) => <option value={value} key={value}>{value === 0 ? 'Specials' : `Season ${value}`}</option>)}
      </select></label></div>
      {episodes.length ? episodes.map((video) => <button className="stremio-episode" key={video.id} disabled={future(video)} onClick={() => { tick(); setEpisode(video); }} aria-label={`Episode ${video.episode}: ${video.title}`}>
        {video.thumbnail && <img src={video.thumbnail} alt="" loading="lazy" />}
        <span><small>EPISODE {video.episode}</small><b>{video.title}</b>{video.description && <span>{video.description}</span>}{future(video) && <small>Available {new Date(video.released).toLocaleDateString()}</small>}</span><i aria-hidden="true">›</i>
      </button>) : <p className="stremio-search-status">No episodes are available for this season yet.</p>}
    </section>}
    {episode && <section className="stremio-sources" aria-label="Stream sources" aria-busy={sources.status === 'loading'}>
      <div className="stremio-section-heading"><h3>Choose a source</h3>{sources.status === 'ready' && <span>{sources.items?.length || 0} sources</span>}</div>
      <p className="stremio-source-notice">Torrentio · Real-Debrid only</p>
      {sources.status === 'loading' && <p className="stremio-search-status" role="status">Finding your Real-Debrid sources…</p>}
      {sources.status === 'error' && <div className="stremio-search-status" role="status"><p>{sources.error}</p><button className="stremio-connect-button" onClick={() => setSourceAttempt((value) => value + 1)}>Try again</button></div>}
      {sources.status === 'ready' && <>
        {sources.failed > 0 && <p className="stremio-source-notice" role="status">Some add-ons didn’t respond. You can choose an available source or load them again.</p>}
        {sources.items?.length > 6 && <input className="stremio-source-filter" type="search" aria-label="Filter sources" placeholder="Filter by quality, language or source" value={filter} maxLength={100} onChange={(event) => setFilter(event.target.value)} />}
        {filtered.map((source) => <button className="stremio-source" key={source.id} disabled={Boolean(opening) || !source.playable} onClick={() => play(source)} aria-label={`Play ${source.name} from ${source.provider}`}>
          <span><small>{source.provider} · {source.kind}</small><b>{source.name}</b>{source.description && <span>{source.description}</span>}{source.reason && <em>{source.reason}</em>}</span>
          <i aria-hidden="true">{opening === source.id ? '…' : '▶'}</i>
        </button>)}
        {!filtered.length && <p className="stremio-search-status">{filter ? 'No sources match that filter.' : 'No Torrentio Real-Debrid sources were found. Try again, or check Torrentio’s Real-Debrid connection in Stremio.'}</p>}
        <button className="stremio-source-reload" disabled={Boolean(opening)} onClick={() => setSourceAttempt((value) => value + 1)}>Load sources again</button>
      </>}
      {opening && <p className="stremio-search-status" role="status">Opening your chosen source in Stremio…</p>}
      {playError && <p className="stremio-error" role="status">{playError}</p>}
    </section>}
    <button className="stremio-tv-fallback" disabled={Boolean(opening)} onClick={() => { tick(); onOpenTV(item); }}>Open {episode && !movie ? 'show' : 'title'} in Stremio on TV</button>
  </div>;
}
