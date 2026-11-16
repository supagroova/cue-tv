(function () {
  'use strict';
  var byId = function (id) { return document.getElementById(id); };
  var page = byId('page'), content = byId('content'), sidebar = byId('sidebar');
  var catalogue = { series: [], movies: [], loading: true, failed: false };
  var collections = [];
  var library = { connected: null, items: [], failed: false };
  var currentPage = 'home', featured = null, detailItem = null, lastMainKey = null, detailOrigin = null;
  var searchItems = [], searchQuery = '', searchState = 'empty', searchSequence = 0;
  var hasNavigated = false, toastTimer = null;
  var choice = null, choiceSequence = 0;
  var names = { home: 'Home', search: 'Search', movies: 'Movies', shows: 'TV Shows', library: 'Library', apps: 'Apps', settings: 'Settings' };
  var reduceMotion = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var scrollBehavior = reduceMotion ? 'auto' : 'smooth';

  function api(method, path, data) {
    return fetch(path, { method: method, headers: data ? { 'Content-Type': 'application/json' } : {},
      body: data ? JSON.stringify(data) : undefined, credentials: 'same-origin' })
      .then(function (r) { return r.json().then(function (j) { if (!r.ok || j.ok === false) throw new Error(j.error || 'Please try again.'); return j; }); });
  }
  function node(tag, className, text) {
    var e = document.createElement(tag); if (className) e.className = className;
    if (text !== undefined) e.textContent = text; return e;
  }
  function icon(name) {
    var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('class', 'icon'); svg.setAttribute('aria-hidden', 'true');
    var use = document.createElementNS('http://www.w3.org/2000/svg', 'use'); use.setAttribute('href', '#i-' + name); svg.appendChild(use); return svg;
  }
  function message(text) {
    clearTimeout(toastTimer); var e = byId('notice'); e.textContent = text || ''; e.hidden = !text;
    if (text) toastTimer = setTimeout(function () { e.hidden = true; }, 6500);
  }
  function artwork(item) { return item && (item.background || item.poster); }
  function applyImage(e, url) {
    e.style.backgroundImage = url && /^https:\/\//.test(url) ? 'url(' + JSON.stringify(url) + ')' : '';
  }
  function metadata(item) { return [item.year, item.type === 'series' ? 'TV Series' : 'Movie'].filter(Boolean).join(' · '); }
  function action(label, name, on, primary, key) {
    var b = node('button', 'action' + (primary ? ' primary' : ''));
    if (name) b.appendChild(icon(name)); b.appendChild(document.createTextNode(label));
    if (key) b.dataset.focusKey = key;
    b.addEventListener('click', on); return b;
  }
  function openInStremio(item) {
    if (!item) return home();
    if (!detailItem || detailItem.id !== item.id || detailItem.type !== item.type) openDetail(item);
    openChoices(item);
  }
  function openApp(pkg) {
    try { if (window.TVOS && window.TVOS.launch(pkg)) return; } catch (_) {}
    // The normal browser preview uses the same authenticated launch route as the phone.
    api('POST', '/api/launch', { app: pkg }).catch(function (e) { message(e.message || 'Could not open this app.'); });
  }
  function openDetail(item) {
    hideChoices();
    detailItem = item; detailOrigin = document.activeElement && document.activeElement.dataset.focusKey;
    byId('detail-title').textContent = item.title; byId('detail-meta').textContent = metadata(item);
    byId('detail-description').textContent = item.description || 'Choose an episode and a source to watch in Cue TV.';
    applyImage(byId('detail-art'), artwork(item));
    byId('detail').hidden = false; page.inert = true; sidebar.inert = true;
    page.setAttribute('aria-hidden', 'true'); sidebar.setAttribute('aria-hidden', 'true');
    byId('detail-open').focus();
  }
  function closeDetail(restore) {
    if (!detailItem) return;
    hideChoices();
    detailItem = null; byId('detail').hidden = true; page.inert = false; sidebar.inert = false;
    page.removeAttribute('aria-hidden'); sidebar.removeAttribute('aria-hidden');
    if (restore !== false) focusKey(detailOrigin) || focusMain();
  }
  function hideChoices() {
    choiceSequence++; choice = null;
    byId('detail-choices').hidden = true; byId('detail-choices').replaceChildren();
    document.querySelector('.detail-copy').hidden = false;
  }
  function choiceHeader(title) {
    var panel = byId('detail-choices'); panel.replaceChildren();
    var heading = node('div', 'choices-header'); heading.appendChild(action('Back', 'back', choiceBack, false));
    heading.appendChild(node('h1', '', title)); panel.appendChild(heading); return panel;
  }
  function choiceBack() {
    if (!choice) return;
    if (choice.stage === 'sources' && choice.item.type === 'series') return renderEpisodes();
    hideChoices(); byId('detail-open').focus();
  }
  function choiceFailure(text, retry) {
    var panel = choiceHeader('Couldn’t load this'); panel.appendChild(node('p', 'empty', text));
    var again = action('Try again', null, retry, true); panel.appendChild(again); again.focus();
  }
  function openChoices(item) {
    var serial = ++choiceSequence;
    var videoParts = (item.video || '').split(':');
    choice = { item: item, stage: 'episodes', season: videoParts.length >= 3 ? Number(videoParts[videoParts.length - 2]) || 1 : 1 };
    document.querySelector('.detail-copy').hidden = true; byId('detail-choices').hidden = false;
    choiceHeader(item.title).appendChild(node('p', 'empty', 'Loading episodes…'));
    api('GET', '/api/stremio/detail?type=' + encodeURIComponent(item.type) + '&id=' + encodeURIComponent(item.id)).then(function (result) {
      if (!choice || serial !== choiceSequence) return;
      choice.info = result;
      if (item.type === 'movie') loadSources(item.id);
      else renderEpisodes();
    }).catch(function (e) { if (choice && serial === choiceSequence) choiceFailure(e.message, function () { openChoices(item); }); });
  }
  function renderEpisodes() {
    if (!choice) return; choiceSequence++; choice.stage = 'episodes';
    var videos = choice.info.videos || [], seasons = Array.from(new Set(videos.map(function (v) { return v.season; }))).sort(function (a, b) { return a - b; });
    if (seasons.indexOf(choice.season) < 0) choice.season = seasons.filter(function (s) { return s > 0; })[0] || seasons[0];
    var panel = choiceHeader(choice.item.title), picker = node('div', 'choices-seasons');
    seasons.forEach(function (season) {
      var button = action(season ? 'Season ' + season : 'Specials', null, function () { choice.season = season; renderEpisodes(); }, season === choice.season);
      picker.appendChild(button);
    }); panel.appendChild(picker);
    var list = node('div', 'choices-list');
    videos.filter(function (v) { return v.season === choice.season; }).forEach(function (video) {
      var button = action(video.episode + '. ' + video.title, null, function () { loadSources(video.id); }, false);
      var released = Date.parse(video.released); button.disabled = isFinite(released) && released > Date.now();
      if (button.disabled) button.appendChild(node('small', '', 'Not released yet'));
      list.appendChild(button);
    }); panel.appendChild(list);
    if (!videos.length) panel.appendChild(node('p', 'empty', 'No episodes are available for this title.'));
    (list.querySelector('button:not(:disabled)') || panel.querySelector('button')).focus();
  }
  function loadSources(video) {
    if (!choice) return; var serial = ++choiceSequence, item = choice.item;
    choice.stage = 'sources'; choice.video = video;
    choiceHeader('Choose a source').appendChild(node('p', 'empty', 'Finding your Real-Debrid sources…'));
    api('GET', '/api/stremio/sources?type=' + encodeURIComponent(item.type) + '&id=' + encodeURIComponent(item.id) + '&video=' + encodeURIComponent(video)).then(function (result) {
      if (!choice || serial !== choiceSequence) return;
      var panel = choiceHeader('Choose a source'), list = node('div', 'choices-list');
      (result.items || []).forEach(function (source) {
        var button = action(source.name, 'play', function () {
          button.disabled = true; message('Opening your source…');
          api('POST', '/api/stremio/play', { id: source.id }).then(function () { message(''); }).catch(function (e) { message(e.message); }).finally(function () { button.disabled = !source.playable; });
        }, false);
        button.disabled = !source.playable;
        button.appendChild(node('small', '', source.description || source.reason || 'Real-Debrid'));
        list.appendChild(button);
      });
      panel.appendChild(list);
      if (!result.items || !result.items.length) { panel.appendChild(node('p', 'empty', 'No Real-Debrid Torrentio sources are available.')); panel.appendChild(action('Try again', null, function () { loadSources(video); }, true)); }
      (list.querySelector('button:not(:disabled)') || panel.querySelector('button')).focus();
    }).catch(function (e) { if (choice && serial === choiceSequence) choiceFailure(e.message, function () { loadSources(video); }); });
  }
  function card(item) {
    var b = node('button', 'card'); b.dataset.focusKey = 'title:' + item.type + ':' + item.id;
    b.setAttribute('aria-label', item.title + ', ' + (item.type === 'series' ? 'TV series' : 'movie') + ', show details');
    var art = node('span', 'art'), fallback = node('span', 'art-fallback', item.title); art.appendChild(fallback);
    var url = artwork(item);
    if (url && /^https:\/\//.test(url)) {
      var img = node('img'); img.alt = ''; img.loading = 'lazy'; img.src = url;
      img.addEventListener('load', function () { fallback.hidden = true; });
      img.addEventListener('error', function () { img.remove(); }); art.appendChild(img);
    }
    if (typeof item.progress === 'number' && item.progress > 0) {
      var track = node('span', 'progress'), fill = node('i'); fill.style.width = Math.round(Math.min(1, item.progress) * 100) + '%'; track.appendChild(fill); art.appendChild(track);
    }
    b.appendChild(art); b.appendChild(node('span', 'card-title', item.title));
    b.appendChild(node('span', 'card-meta', item.inProgress ? 'Continue watching' : (item.year || (item.type === 'series' ? 'TV Series' : 'Movie'))));
    b.addEventListener('click', function () { openDetail(item); }); return b;
  }
  function shelf(title, items) {
    if (!items || !items.length) return null;
    var section = node('section', 'section'); section.setAttribute('aria-label', title);
    section.appendChild(node('h2', '', title)); var rail = node('div', 'rail'); rail.setAttribute('data-nav-group', '');
    items.forEach(function (item) { rail.appendChild(card(item)); }); section.appendChild(rail); return section;
  }
  function append(parent, children) { children.forEach(function (e) { if (e) parent.appendChild(e); }); }
  function loading(parent) {
    var rail = node('div', 'loading-rail'); rail.setAttribute('aria-label', 'Loading titles');
    for (var i = 0; i < 4; i++) rail.appendChild(node('div', 'loading-card'));
    parent.appendChild(rail);
  }
  function accountCard() {
    var section = node('section', 'connection-card');
    section.appendChild(node('h2', '', library.connected ? 'Your library is ready for its first show.' : 'Pick up where you left off.'));
    section.appendChild(node('p', '', library.connected ? 'Your Stremio viewing history will appear here as you watch.' : 'Connect the Stremio account you use on this TV to bring in Continue Watching and your viewing history.'));
    var actions = node('div', 'actions'); actions.setAttribute('data-nav-group', '');
    actions.appendChild(action(library.connected ? 'Open Stremio' : 'How to connect', library.connected ? 'play' : 'library', function () {
      if (library.connected) openApp('com.stremio.one'); else message('On your iPhone remote, choose Watch → Connect Stremio, then sign in with the same Stremio account as this TV.');
    }, true, 'account')); section.appendChild(actions); return section;
  }
  function hero(item, context) {
    var section = node('section', 'hero'), art = node('div', 'hero-art'); applyImage(art, artwork(item));
    section.appendChild(art); section.appendChild(node('div', 'hero-scrim'));
    var copy = node('div', 'hero-copy'); copy.appendChild(node('p', 'hero-context', context || 'Stremio'));
    copy.appendChild(node('h1', '', item ? item.title : 'What shall we watch?'));
    if (item) copy.appendChild(node('p', 'metadata', metadata(item)));
    copy.appendChild(node('p', 'hero-description', item && item.description ? item.description : 'Your movies, shows and favourite apps, together on your TV.'));
    var actions = node('div', 'actions'); actions.setAttribute('data-nav-group', '');
    actions.appendChild(action(item ? (item.type === 'series' ? 'Episodes & Sources' : 'Choose a source') : 'Watch Now', 'play', function () { openInStremio(item); }, true, 'hero-watch'));
    actions.appendChild(action('YouTube', 'youtube', function () { openApp('com.google.android.youtube.tv'); }, false, 'hero-youtube'));
    copy.appendChild(actions); section.appendChild(copy); return section;
  }
  function subpage(title, subtitle) {
    var wrap = node('div', 'subpage'); wrap.appendChild(node('h1', '', title));
    if (subtitle) wrap.appendChild(node('p', 'page-subtitle', subtitle)); content.appendChild(wrap); return wrap;
  }
  function catalogueError(parent) {
    var box = node('div', 'connection-card'); box.appendChild(node('h2', '', 'The catalogue couldn’t load.'));
    box.appendChild(node('p', '', 'You can retry, or open Stremio to keep watching.'));
    var row = node('div', 'actions'); row.setAttribute('data-nav-group', '');
    row.appendChild(action('Try again', null, refreshCatalogue, true, 'retry-catalogue'));
    row.appendChild(action('Open Stremio', 'play', function () { openApp('com.stremio.one'); }, false, 'fallback-stremio'));
    box.appendChild(row); parent.appendChild(box);
  }
  function renderHome(type) {
    var continueItems = library.items.filter(function (it) { return it.inProgress && (!type || it.type === type); });
    var series = catalogue.series || [], movies = catalogue.movies || [];
    var top = continueItems[0] || (type === 'movie' ? movies[0] : series[0]) || movies[0];
    if (!featured || type) featured = top;
    var chosen = type ? top : featured;
    var continuing = chosen && continueItems.some(function (i) { return i.id === chosen.id && i.type === chosen.type; });
    content.appendChild(hero(chosen, continuing ? 'Continue watching' : 'Popular on Stremio'));
    if (catalogue.loading) { var placeholder = node('div', 'section'); placeholder.appendChild(node('h2', '', 'Discover something new')); loading(placeholder); content.appendChild(placeholder); return; }
    if (catalogue.failed && !series.length && !movies.length) { var err = node('div', 'section'); catalogueError(err); content.appendChild(err); return; }
    append(content, [
      shelf('Continue Watching', continueItems),
      !type || type === 'series' ? shelf('Trending TV Shows', catalogue.trendingSeries) : null,
      !type || type === 'movie' ? shelf('Trending Movies', catalogue.trendingMovies) : null,
      !type || type === 'series' ? shelf('Popular TV Shows', series) : null,
      !type || type === 'movie' ? shelf('Popular Movies', movies) : null
    ]);
    collections.forEach(function (row) { if (!type || row.type === type) append(content, [shelf(row.title, row.items)]); });
  }
  function renderLibrary() {
    var wrap = subpage('Library', 'Your watching history, from Stremio.');
    if (library.failed) {
      wrap.appendChild(node('p', 'empty', 'Your history couldn’t refresh. Try again in a moment.'));
      var retry = node('div', 'actions'); retry.setAttribute('data-nav-group', ''); retry.appendChild(action('Refresh history', null, refreshLibrary, true, 'retry-library')); wrap.appendChild(retry);
    }
    if (!library.connected || !library.items.length) wrap.appendChild(accountCard());
    append(wrap, [shelf('Continue Watching', library.items.filter(function (i) { return i.inProgress; })), shelf('Recently Watched', library.items.filter(function (i) { return !i.inProgress; }))]);
  }
  function nativeApps() {
    try { return JSON.parse(window.TVOS.apps()); } catch (_) { return [{ package: 'com.stremio.one', label: 'Stremio' }, { package: 'com.google.android.youtube.tv', label: 'YouTube' }]; }
  }
  function renderApps() {
    var wrap = subpage('Apps'), grid = node('div', 'apps-grid'), apps = nativeApps();
    apps = apps.filter(function (a) { return a.package && a.package !== 'dev.couchtv.ontv' && a.package !== 'com.google.android.apps.tv.launcherx'; });
    var favourites = ['com.stremio.one', 'com.google.android.youtube.tv'];
    apps.sort(function (a, b) { var ai = favourites.indexOf(a.package), bi = favourites.indexOf(b.package); return (ai < 0 ? 99 : ai) - (bi < 0 ? 99 : bi) || String(a.label).localeCompare(String(b.label)); });
    var row;
    apps.forEach(function (a, index) {
      if (index % 5 === 0) { row = node('div', 'app-grid-row'); row.setAttribute('data-nav-group', ''); row.style.display = 'contents'; grid.appendChild(row); }
      var b = node('button', 'app-tile'), image = node('span', 'app-image'), img = node('img');
      b.dataset.focusKey = 'app:' + a.package; b.setAttribute('aria-label', 'Open ' + a.label); img.alt = '';
      img.style.width = '100%'; img.style.height = '100%'; img.style.objectFit = 'cover'; img.style.borderRadius = '0';
      img.src = 'http://tvos.invalid/banner/' + encodeURIComponent(a.package);
      img.onerror = function () {
        if (!img.dataset.fallback) { img.dataset.fallback = 'icon'; img.src = 'http://tvos.invalid/icon/' + encodeURIComponent(a.package); img.style.width = '4rem'; img.style.height = '4rem'; img.style.objectFit = 'contain'; }
        else { img.remove(); image.appendChild(icon(a.package === 'com.google.android.youtube.tv' ? 'youtube' : 'play')); }
      };
      image.appendChild(img); b.appendChild(image); b.appendChild(node('span', 'app-label', a.label || a.package));
      b.addEventListener('click', function () { openApp(a.package); }); row.appendChild(b);
    }); wrap.appendChild(grid);
  }
  function renderSettings() {
    var wrap = subpage('Settings', 'Make this TV work for your household.');
    var row = node('div', 'actions'); row.setAttribute('data-nav-group', '');
    row.appendChild(action('TV settings', 'settings', function () { if (window.TVOS) window.TVOS.openSettings(); }, true, 'tv-settings'));
    row.appendChild(action('Couch TV setup', 'shows', function () { openApp('dev.couchtv.ontv'); }, false, 'couch-setup'));
    row.appendChild(action('Home settings', 'home', function () { location.href = '/tv/index.html'; }, false, 'home-settings'));
    wrap.appendChild(row);
  }
  function runSearch(query) {
    searchQuery = query.trim(); var seq = ++searchSequence;
    if (!searchQuery) { searchItems = []; searchState = 'empty'; renderSearchResults(); return; }
    searchState = 'loading'; renderSearchResults();
    api('GET', '/api/stremio/search?q=' + encodeURIComponent(searchQuery)).then(function (j) {
      if (seq !== searchSequence) return; searchItems = j.items || []; searchState = 'ready'; renderSearchResults();
    }).catch(function () { if (seq !== searchSequence) return; searchState = 'error'; renderSearchResults(); });
  }
  function renderSearchResults() {
    var results = byId('search-results'); if (!results) return; results.replaceChildren();
    if (searchState === 'loading') return loading(results);
    if (searchState === 'error') { results.appendChild(node('p', 'empty', 'Search couldn’t load. Choose Search to try again.')); return; }
    if (searchState === 'empty') { results.appendChild(node('p', 'empty', 'Search movies and TV shows by name.')); return; }
    if (!searchItems.length) { results.appendChild(node('p', 'empty', 'No titles found for “' + searchQuery + '”. Try a different name.')); return; }
    append(results, [shelf('TV Shows', searchItems.filter(function (i) { return i.type === 'series'; })), shelf('Movies', searchItems.filter(function (i) { return i.type === 'movie'; }))]);
  }
  function renderSearch() {
    var wrap = subpage('Search'), form = node('form', 'search-form'); form.setAttribute('data-nav-group', '');
    var input = node('input'); input.id = 'search-input'; input.type = 'search'; input.placeholder = 'Movies and TV shows'; input.value = searchQuery;
    input.autocomplete = 'off'; input.maxLength = 100; input.setAttribute('aria-label', 'Search movies and TV shows'); input.dataset.focusKey = 'search-input';
    form.addEventListener('submit', function (e) { e.preventDefault(); runSearch(input.value); });
    var button = node('button', 'action primary', 'Search'); button.type = 'submit'; button.dataset.focusKey = 'search-submit';
    form.appendChild(input); form.appendChild(button); wrap.appendChild(form);
    var results = node('div'); results.id = 'search-results'; results.setAttribute('aria-live', 'polite'); wrap.appendChild(results); renderSearchResults();
  }
  function focusKey(key) {
    if (!key) return false;
    var found = Array.from(content.querySelectorAll('[data-focus-key]')).find(function (e) { return e.dataset.focusKey === key; });
    if (!found) return false; found.focus({ preventScroll: true }); return true;
  }
  function focusMain() {
    if (lastMainKey && focusKey(lastMainKey)) return;
    var first = content.querySelector('button:not(:disabled), input:not(:disabled)'); if (first) first.focus({ preventScroll: true });
  }
  function render() {
    var oldFocus = content.contains(document.activeElement) ? document.activeElement.dataset.focusKey : null;
    var oldScroll = page.scrollTop; content.replaceChildren();
    if (currentPage === 'home') renderHome();
    else if (currentPage === 'movies') renderHome('movie');
    else if (currentPage === 'shows') renderHome('series');
    else if (currentPage === 'library') renderLibrary();
    else if (currentPage === 'apps') renderApps();
    else if (currentPage === 'search') renderSearch();
    else renderSettings();
    page.scrollTop = oldScroll;
    if (oldFocus && !detailItem) focusKey(oldFocus) || focusMain();
  }
  function showPage(name, fromNav) {
    if (detailItem) closeDetail(false);
    currentPage = name; lastMainKey = null; page.scrollTop = 0; byId('page-name').textContent = names[name];
    document.querySelectorAll('.nav-item[data-page]').forEach(function (b) {
      b.classList.toggle('selected', b.dataset.page === name);
      if (b.dataset.page === name) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
    });
    byId('settings').classList.toggle('selected', name === 'settings');
    render(); if (!fromNav) focusMain();
  }
  function refreshCatalogue() {
    api('GET', '/api/stremio/collections').then(function (j) { collections = j.collections || []; if (currentPage !== 'search') render(); }).catch(function () {});
    return api('GET', '/api/stremio/discover').then(function (j) {
      catalogue = Object.assign({ loading: false, failed: false, series: [], movies: [] }, j); if (currentPage !== 'search') render();
    }).catch(function () { catalogue.loading = false; catalogue.failed = true; if (currentPage !== 'search') render(); });
  }
  function refreshLibrary() {
    return api('GET', '/api/stremio/library').then(function (j) {
      library = Object.assign({ items: [], failed: false }, j); if (currentPage !== 'search') render();
    }).catch(function () { library.failed = true; if (currentPage === 'library') render(); });
  }
  function resume() {
    Promise.all([api('GET', '/api/config'), api('GET', '/api/who')]).then(function (values) {
      if ((values[0].people || []).some(function (p) { return p.id === values[1].who && p.kids_mode; })) location.href = '/tv/index.html';
      else { refreshLibrary(); refreshCatalogue(); }
    }).catch(function () {});
  }
  function navForPage() { return document.querySelector('.nav-item[data-page="' + currentPage + '"]') || byId('settings'); }
  function back() {
    if (choice) return choiceBack();
    if (detailItem) return closeDetail();
    if (sidebar.contains(document.activeElement)) { if (currentPage !== 'home') showPage('home', true); else focusMain(); }
    else navForPage().focus();
  }
  function home() { showPage('home'); page.scrollTo({ top: 0, behavior: scrollBehavior }); }
  function visibleFocusables(group) { return Array.from(group.querySelectorAll('button:not(:disabled),input:not(:disabled)')).filter(function (e) { return !e.hidden && e.getClientRects().length; }); }
  function mainGroups() { return Array.from(content.querySelectorAll('[data-nav-group]')).map(visibleFocusables).filter(function (g) { return g.length; }); }
  function focusMove(target) {
    target.focus({ preventScroll: true });
    if (!sidebar.contains(target)) target.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: scrollBehavior });
  }
  document.addEventListener('focusin', function (e) { if (content.contains(e.target) && e.target.dataset.focusKey) lastMainKey = e.target.dataset.focusKey; });
  document.addEventListener('keydown', function (e) {
    var k = e.key, code = e.keyCode, current = document.activeElement;
    if (k === 'Escape' || k === 'BrowserBack') { e.preventDefault(); back(); return; }
    var dx = k === 'ArrowLeft' || code === 21 ? -1 : k === 'ArrowRight' || code === 22 ? 1 : 0;
    var dy = k === 'ArrowUp' || code === 19 ? -1 : k === 'ArrowDown' || code === 20 ? 1 : 0;
    if (dx || dy) {
      hasNavigated = true;
      if (current && current.tagName === 'INPUT' && !dy) return;
      e.preventDefault();
      if (choice) {
        var controls = visibleFocusables(byId('detail-choices')), index = controls.indexOf(current);
        focusMove(controls[Math.max(0, Math.min(controls.length - 1, index + (dy || dx)))]); return;
      }
      if (detailItem) { focusMove(dy < 0 || dx < 0 ? byId('detail-back') : byId('detail-open')); return; }
      if (sidebar.contains(current)) {
        var nav = Array.from(sidebar.querySelectorAll('button')), ni = nav.indexOf(current);
        if (dx > 0) focusMain(); else if (dy) focusMove(nav[Math.max(0, Math.min(nav.length - 1, ni + dy))]); return;
      }
      var groups = mainGroups(), gi = groups.findIndex(function (g) { return g.indexOf(current) >= 0; });
      if (gi < 0) { focusMain(); return; }
      var at = groups[gi].indexOf(current);
      if (dx < 0 && at === 0) { navForPage().focus(); return; }
      if (dx) { focusMove(groups[gi][Math.max(0, Math.min(groups[gi].length - 1, at + dx))]); return; }
      var next = groups[gi + dy]; if (!next) return;
      var box = current.getBoundingClientRect(), center = box.left + box.width / 2;
      var closest = next.reduce(function (best, candidate) { var b = candidate.getBoundingClientRect(); var d = Math.abs(b.left + b.width / 2 - center); return !best || d < best.d ? { e: candidate, d: d } : best; }, null);
      if (closest) focusMove(closest.e); return;
    }
    if (k === 'Enter' || code === 23 || code === 66 || k === ' ') {
      hasNavigated = true;
      if (current && current.tagName === 'BUTTON') { e.preventDefault(); current.click(); }
    }
  });
  document.querySelectorAll('.nav-item[data-page]').forEach(function (b) { b.addEventListener('click', function () { showPage(b.dataset.page); }); });
  document.querySelector('[data-launch]').addEventListener('click', function () { openApp(this.dataset.launch); });
  byId('settings').addEventListener('click', function () { showPage('settings'); });
  byId('detail-back').addEventListener('click', function () { if (choice) choiceBack(); else closeDetail(); });
  byId('detail-open').addEventListener('click', function () { openInStremio(detailItem); });
  document.querySelectorAll('svg.icon').forEach(function (s) { s.setAttribute('aria-hidden', 'true'); });
  function ready() { if (!hasNavigated) focusMain(); }
  window.tv = { home: home, back: back, resume: resume, pause: function () {}, ready: ready, settingsKey: function () { showPage('settings'); }, openTitle: function (type, id) {
    api('GET', '/api/stremio/detail?type=' + encodeURIComponent(type) + '&id=' + encodeURIComponent(id)).then(openDetail).catch(function (e) { message(e.message); });
  } };
  setInterval(function () { api('GET', '/api/stremio/status').then(function (j) { if (j.connected !== library.connected) { refreshLibrary(); refreshCatalogue(); } }).catch(function () {}); }, 15000);
  function clock() { byId('clock').textContent = new Date().toLocaleTimeString('en-AU', { hour: 'numeric', minute: '2-digit' }); }
  clock(); setInterval(clock, 60_000); render(); ready(); refreshCatalogue(); refreshLibrary();
})();
