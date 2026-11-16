// Local presentation only. No feed, account, cookie or credential data leaves this web view.
(() => {
  if (location.protocol !== 'https:' || !['youtube.com', 'www.youtube.com', 'm.youtube.com'].includes(location.hostname)) return;
  const css = `
    html[data-cue-feed="home"] { color-scheme: dark; background: #08080a !important; }
    html[data-cue-feed="home"] body { background: #08080a !important; color: #f5f5f7 !important; font-family: -apple-system, BlinkMacSystemFont, sans-serif !important; }
    html[data-cue-feed="home"] ytm-mobile-topbar-renderer,
    html[data-cue-feed="home"] ytm-pivot-bar-renderer,
    html[data-cue-feed="home"] ytm-feed-filter-chip-bar-renderer,
    html[data-cue-feed="home"] .feed-filter-chip-bar { display: none !important; }
    html[data-cue-feed="home"] ytm-app { padding-top: 0 !important; padding-bottom: 0 !important; }
    html[data-cue-feed="home"] ytm-browse,
    html[data-cue-feed="home"] ytm-section-list-renderer,
    html[data-cue-feed="home"] ytm-item-section-renderer,
    html[data-cue-feed="home"] ytm-rich-item-renderer,
    html[data-cue-feed="home"] .rich-item-renderer-contents { margin: 0 !important; padding: 0 !important; background: transparent !important; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer { display: block !important; margin: 0 !important; padding: 12px 4px !important; border-bottom: 1px solid rgba(255,255,255,.08); background: transparent !important; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer ytm-media-item { display: grid !important; grid-template-columns: minmax(112px, 38%) minmax(0, 1fr); align-items: start; gap: 12px; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer .media-item-thumbnail-container { display: block !important; position: relative !important; width: 100% !important; height: auto !important; padding: 0 !important; aspect-ratio: 16/9; border-radius: 10px; overflow: hidden; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer .video-thumbnail-container-large { display: block !important; width: 100% !important; height: 100% !important; padding: 0 !important; aspect-ratio: 16/9; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer .media-item-details { min-width: 0 !important; padding: 0 !important; margin: 0 !important; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer .media-item-headline { margin: 0 !important; font: 600 15px/1.3 -apple-system, BlinkMacSystemFont, sans-serif !important; -webkit-line-clamp: 3 !important; }
    html[data-cue-feed="home"] ytm-video-with-context-renderer .media-item-byline { font-size: 12px !important; line-height: 1.35 !important; color: #a1a1a8 !important; }
    html[data-cue-feed="home"] #video-preview-portal,
    html[data-cue-feed="home"] ytm-video-preview { display: none !important; }
    html[data-cue-feed="home"] button { touch-action: manipulation; }
    @media (prefers-reduced-motion: reduce) { html[data-cue-feed="home"] * { scroll-behavior: auto !important; } }
  `;
  const style = document.createElement('style'); style.id = 'cue-feed-appearance'; style.textContent = css;
  (document.head || document.documentElement).append(style);
  const update = () => {
    document.documentElement.dataset.cueFeed = location.pathname === '/' ? 'home' : 'browse';
    document.querySelectorAll('video, audio').forEach(media => media.pause());
  };
  // This app chooses TV playback. Website previews should remain still, even after scrolling.
  document.addEventListener('play', event => {
    if (event.target instanceof HTMLMediaElement) event.target.pause();
  }, true);
  window.addEventListener('pageshow', update);
  window.addEventListener('popstate', update);
  window.cueFeedAppearance = { update };
  update();
})();
