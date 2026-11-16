import { useEffect, useRef, useState } from 'react';
import { tick } from './haptics.js';

export function YouTubeHomeFeed({ active }) {
  const space = useRef(null);
  const [feeds, setFeeds] = useState(null);
  useEffect(() => {
    let live = true;
    const update = (event) => { if (live && Array.isArray(event.detail?.profiles)) setFeeds(event.detail); };
    window.addEventListener('tv:youtube-feeds', update);
    window.couchNative?.getYouTubeFeeds?.().then((value) => { if (live) setFeeds(value); }).catch(() => {});
    return () => { live = false; window.removeEventListener('tv:youtube-feeds', update); };
  }, []);
  useEffect(() => {
    const native = window.couchNative;
    if (!native?.canInlineYouTube || !native.setYouTubeFeed) return undefined;
    let frame = 0;
    let previous = '';
    const send = (payload) => {
      const signature = JSON.stringify(payload);
      if (signature !== previous) { previous = signature; native.setYouTubeFeed(payload); }
    };
    const report = () => {
      frame = 0;
      const node = space.current;
      const watch = node?.closest('.watch');
      if (!active || !node || !watch || document.visibilityState === 'hidden') { send({ visible: false }); return; }
      const full = node.getBoundingClientRect();
      const parent = watch.getBoundingClientRect();
      const x = Math.max(0, full.left, parent.left);
      const y = Math.max(0, full.top, parent.top);
      const right = Math.min(innerWidth, full.right, parent.right);
      const bottom = Math.min(innerHeight, full.bottom, parent.bottom);
      if (right <= x || bottom <= y) { send({ visible: false }); return; }
      send({ visible: true, frame: { x: full.x, y: full.y, width: full.width, height: full.height },
        clip: { x, y, width: right - x, height: bottom - y }, viewport: { width: innerWidth, height: innerHeight } });
    };
    const schedule = () => { if (!frame) frame = requestAnimationFrame(report); };
    const observer = new ResizeObserver(schedule);
    if (space.current) observer.observe(space.current);
    const watch = space.current?.closest('.watch');
    if (watch) observer.observe(watch);
    const browser = space.current?.closest('.youtube-browser');
    if (browser) observer.observe(browser);
    document.addEventListener('scroll', schedule, true);
    document.addEventListener('visibilitychange', schedule);
    window.addEventListener('resize', schedule);
    window.addEventListener('tv:resume', schedule);
    // The existing horizontal pager uses a short smooth transition; follow each scroll event.
    schedule();
    return () => {
      cancelAnimationFrame(frame); observer.disconnect();
      document.removeEventListener('scroll', schedule, true);
      document.removeEventListener('visibilitychange', schedule);
      window.removeEventListener('resize', schedule); window.removeEventListener('tv:resume', schedule);
      native.setYouTubeFeed({ visible: false });
    };
  }, [active]);
  return <section className="youtube-inline-home" aria-label="Your YouTube recommendations">
    <header className="youtube-feed-heading"><h3>YouTube Home</h3>
      {feeds?.profiles?.length > 1 && <select aria-label="YouTube feed" value={feeds.selected} onChange={(event) => { tick(); window.couchNative?.selectYouTubeFeed?.(event.target.value); }}>
        {feeds.profiles.map((feed) => <option value={feed.id} key={feed.id}>{feed.name}</option>)}
      </select>}
    </header>
    <div ref={space} className="youtube-inline-space"><p>Loading your YouTube Home…</p></div>
  </section>;
}
