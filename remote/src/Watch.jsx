import { useEffect, useState } from 'react';
import * as Ic from './icons.jsx';
import { tick } from './haptics.js';
import { logoFor } from './appLogos.js';
import { t, tBridge } from './i18n.js';
import { StremioBrowser } from './StremioBrowser.jsx';
import { YouTubeBrowser } from './YouTubeBrowser.jsx';

// Names that fit under an icon, like the iPhone home screen.
const SHORT = { 'sbs-on-demand': 'SBS', 'prime-video': 'Prime Video', 'disney-plus': 'Disney+', 'paramount-plus': 'Paramount+', 'foxtel-go': 'Foxtel', 'hbo-max': 'HBO Max' };

// An app's icon, plus its clear glyph (clearicon.js) for the glass look (#25): app.css shows the
// glyph instead of the icon while Glass app icons is on, and the icon again while it is pressed.
function AppIcon({ src }) {
  const [glyph, setGlyph] = useState(() => window.ClearIcon?.cached(src) || '');
  useEffect(() => {
    let live = true;
    window.ClearIcon?.get(src, (g) => { if (live) setGlyph(g || ''); });
    return () => { live = false; };
  }, [src]);
  return (
    <span className={'app-icon' + (glyph ? ' clear' : '')}>
      <img src={src} alt="" />
      {glyph && <img className="glyph" src={glyph} alt="" />}
    </span>
  );
}

function episode(s) {
  return [s.season && `S${s.season}`, s.episode && `E${s.episode}`].filter(Boolean).join(' · ');
}

function Poster({ show, sub, onPlay, onRemove, dim }) {
  const poster = (
    <button className={`poster ${dim ? 'dim' : ''}`} onClick={() => { tick(); onPlay(show); }}>
      <div className="poster-art">{show.poster ? <img src={show.poster} alt="" loading="lazy" /> : <span>{show.title}</span>}</div>
      <b>{show.title}</b>
      <small>{sub}</small>
    </button>
  );
  if (!onRemove) return poster;
  return (
    <div className="poster-wrap">
      {poster}
      <button className="poster-remove" aria-label={t('kw_remove', { title: show.title })} onClick={() => { tick(); onRemove(show); }}><Ic.Close size={14} /></button>
    </div>
  );
}

export function Watch({ pull, people, who, recs, cont: contIn, apps, hidden, screens, serviceName, onPlay, onStremioOpen, onStremioPlaySource, onYouTubeOpen, onRemove, onLaunch, onScreen, wide, active, kidsActive, platform, platformBusy, platformDisabled, onPlatform }) {
  const cont = contIn || [];
  const apple = platform?.active === 'apple';
  const picks = (recs?.picks || []).filter((s) => s.playable !== false);
  const ready = (screens || []).filter((s) => s.ready);
  const [showPicks, setShowPicks] = useState(false);
  const [provider, setProvider] = useState('stremio');
  useEffect(() => setShowPicks(false), [who]);
  return (
    <div className={`watch ${wide ? 'wide' : ''}`}>
      {!wide && <div className="pull-zone watch-pull" {...pull} aria-label="TV controls" />}
      {!kidsActive && <>
        <div className="watch-providers" role="group" aria-label="Search platform">
          {['stremio', 'youtube'].map((id) => <button key={id} aria-pressed={provider === id} onClick={() => { tick(); setProvider(id); }}>{id === 'stremio' ? 'Stremio' : 'YouTube'}</button>)}
        </div>
        <div hidden={provider !== 'stremio'}><StremioBrowser onOpen={onStremioOpen} onPlaySource={onStremioPlaySource} onLaunch={onLaunch} active={active && provider === 'stremio'} platform={platform} platformBusy={platformBusy} platformDisabled={platformDisabled} onPlatform={onPlatform} /></div>
        <div hidden={provider !== 'youtube'}><YouTubeBrowser active={active && provider === 'youtube'} onOpen={onYouTubeOpen} onLaunch={onLaunch} platform={platform} platformBusy={platformBusy} platformDisabled={platformDisabled} onPlatform={onPlatform} /></div>
      </>}

      {!apple && cont.length > 0 && <section>
        <h3>On this TV{who !== 'both' ? <span className="h3-who"> · {(people || []).find((p) => p.id === who)?.name || ''}</span> : null}</h3>
        {cont.length ? (
          <div className="shelf">
            {cont.map((s) => <Poster key={s.id || `${s.title}-${s.service}`} show={s} onPlay={onPlay} onRemove={s.id ? onRemove : null}
              sub={[episode(s) || s.episode_title, s.app?.name || serviceName(s.service)].filter(Boolean).join(' · ')} />)}
          </div>
        ) : <p className="empty">{contIn === false ? t('couldnt_load') : contIn ? t('shows_wait') : t('loading')}</p>}
      </section>}

      {kidsActive && <section>
        <h3>{t('apps')}</h3>
        {apps === null && <p className="empty">{t('loading')}</p>}
        {apps === false && <p className="empty">{t('couldnt_load_apps')}</p>}
        <div className="apps">
          {(apps || []).filter((a) => a.installed !== false && !['settings', 'home', 'moonlight'].includes(a.id) && !(hidden || []).includes(a.package) && (!apple || ['stremio', 'youtube'].includes(a.id))).map((a) => (
            <button key={a.id} className="app-tile" style={{ '--tile': a.colour || '#3a3a3c' }} onClick={() => { tick(); onLaunch(a); }}>
              {logoFor(a) ? <AppIcon src={logoFor(a)} /> : <span className="app-icon"><span className="app-icon-name">{a.name}</span></span>}
              <span className="app-label">{SHORT[a.id] || a.name}</span>
            </button>
          ))}
        </div>
      </section>}

      {!apple && <section>
        <h3>{t('ideas_tonight')}</h3>
        {!showPicks ? (
          <button className="row-btn cell" onClick={() => { tick(); setShowPicks(true); }}>
            <span className="row-icon"><Ic.Sparkle /></span>
            <span className="row-text"><b>{t('suggest')}</b><small>{t('suggest_sub')}</small></span>
            <Ic.Chevron className="row-chev" />
          </button>
        ) : picks.length ? (
          <div className="shelf">
            {picks.map((s) => <Poster key={`${s.title}-p`} show={s} onPlay={onPlay} sub={tBridge(s.reason_key, s.reason_vars, s.reason)} />)}
          </div>
        ) : <p className="empty">{recs === false ? t('couldnt_load_ideas') : recs ? t('no_ideas') : t('loading')}</p>}
      </section>}

      {!apple && ready.length > 0 && (
        <section>
          <h3>{t('screens')}</h3>
          {ready.map((s) => (
            <button key={s.id} className={`row-btn cell ${s.ready ? '' : 'not-ready'}`} onClick={() => { tick(); onScreen(s); }}>
              <span className="row-icon"><Ic.Screen /></span>
              <span className="row-text"><b>{s.name}</b><small>{s.ready ? t('screen_sub') : tBridge(s.note_key, s.note_vars, s.note)}</small></span>
              <Ic.Chevron className="row-chev" />
            </button>
          ))}
        </section>
      )}
    </div>
  );
}
