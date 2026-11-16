"""Disposable localhost-only TV API and phone build. Artwork/catalogue fixtures contain public
titles, never household watch history. __fixture controls exist only in this test server."""
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import hashlib
from pathlib import Path
import time
import os
from urllib.parse import urlsplit, parse_qs

ROOT = Path(__file__).resolve().parents[1]
STATE = {'active': 'android', 'home_request': 0, 'open_request': None, 'fail_switch': False, 'events': [],
         'accounts': {}, 'youtube_configured': True, 'youtube_library_error': False, 'youtube_library_empty': False}

def titles(file, kind):
    path = ROOT / 'dist' / file
    if path.exists():
        return [dict(id=x['id'], title=x['name'], type=x['type'], poster=x.get('poster'), background=x.get('background'),
                     description=x.get('description', ''), year=x.get('releaseInfo', '')) for x in json.loads(path.read_text())['metas'][:12]]
    return [dict(id='tt1234', title='Sample ' + kind, type=kind)]
SERIES = titles('demo-series.json', 'series')
MOVIES = titles('demo-movies.json', 'movie')

def platform():
    active = STATE['active']
    return {'active': active, 'home_request': STATE['home_request'], 'open_request': STATE['open_request'], 'label': {'apple': 'Apple TV', 'android': 'Android TV', 'other': 'Other input'}.get(active, 'Checking input'),
            'options': [{'id': 'apple', 'name': 'Apple TV', 'ready': True}, {'id': 'android', 'name': 'Android TV', 'ready': True}]}

class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs): super().__init__(*args, directory=str(ROOT / 'remote/dist'), **kwargs)
    def log_message(self, *args): pass
    def reply(self, value, status=200):
        data = json.dumps(value).encode()
        self.send_response(status); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(data))); self.end_headers()
        self.wfile.write(data)
    def now(self):
        return {'ok': True, 'tv_connected': True, 'power': 'on', 'mode': 'on-tv', 'app': {'id': 'home', 'name': 'TV'},
                'playing': None, 'volume': {'known': True, 'level': 10, 'max': 30}, 'platform': platform()}
    def youtube_client(self):
        return self.headers.get('Authorization', 'Bearer fixture-default').removeprefix('Bearer ')
    def youtube_account(self):
        account = STATE['accounts'].get(self.youtube_client(), {})
        connected = account.get('state') == 'connected'
        return {'configured': STATE['youtube_configured'], 'connected': connected,
                'state': account.get('state', 'disconnected') if STATE['youtube_configured'] else 'setup_required',
                'connection': account.get('connection'), 'account': {'title': account.get('title', 'Sample account')},
                'link': account.get('link')}
    def do_GET(self):
        url = urlsplit(self.path); path = url.path
        if path == '/api/stremio/detail':
            if STATE.get('stremio_detail_error'): return self.reply({'ok': False, 'error': 'Could not load these episodes.'}, 502)
            query = parse_qs(url.query); kind = query.get('type', ['series'])[0]; identity = query.get('id', ['tt1'])[0]
            title = next((dict(value) for value in SERIES + MOVIES if value['id'] == identity), {'id': identity, 'title': 'Fixture title'})
            title.update(type=kind, description='Sample title details for the phone chooser.')
            if identity == SERIES[0]['id']: title['title'] = 'Bluey'
            title['videos'] = [
                {'id': identity + ':1:1', 'season': 1, 'episode': 1, 'title': 'A quiet morning', 'description': 'A sample first episode.', 'released': '2020-01-01T00:00:00Z'},
                {'id': identity + ':1:2', 'season': 1, 'episode': 2, 'title': 'At the park', 'description': 'A sample second episode.', 'released': '2020-01-02T00:00:00Z'},
                {'id': identity + ':2:1', 'season': 2, 'episode': 1, 'title': 'Season two opener', 'description': 'Another sample episode.', 'released': '2021-01-01T00:00:00Z'},
                {'id': identity + ':2:2', 'season': 2, 'episode': 2, 'title': 'Not released yet', 'released': '2999-01-01T00:00:00Z'},
                {'id': identity + ':0:1', 'season': 0, 'episode': 1, 'title': 'Special episode', 'released': '2020-01-01T00:00:00Z'},
            ] if kind == 'series' else []
            return self.reply({'ok': True, **title})
        if path == '/api/stremio/sources':
            if STATE.get('stremio_sources_error'): return self.reply({'ok': False, 'error': 'Could not load your sources.'}, 502)
            query = parse_qs(url.query); identity = query.get('id', ['tt1'])[0]; video = query.get('video', [identity])[0]
            labels = ['4K HDR', '1080p', '720p', '4K Dolby Vision', '1080p HEVC', '720p HEVC', '480p', '1080p Alternate']
            items = []
            for label in labels:
                choice = hashlib.sha256((video + label).encode()).hexdigest()[:32]
                STATE.setdefault('stremio_choices', {})[choice] = {'id': identity, 'video': video, 'quality': label}
                items.append({'id': choice, 'provider': 'Fixture provider', 'name': label, 'description': 'Sample release\n2.4 GB · English', 'kind': 'Torrent', 'playable': True})
            return self.reply({'ok': True, 'items': [] if STATE.get('stremio_sources_empty') else items, 'providers': 2, 'failed': 1 if STATE.get('stremio_provider_failed') else 0})
        routes = {
            '/api/now': self.now(), '/api/platform': {'platform': platform()},
            '/api/health': {'mode': 'on-tv', 'auth': 'ok'}, '/api/config': {'people': [], 'home': {}, 'ask': {'enabled': False}},
            '/api/tvs': {'default': 'living-room', 'tvs': [{'id': 'living-room', 'name': 'Sitting Room TV', 'connected': True, 'paired': True}]},
            '/api/apps': {'apps': [{'id': 'stremio', 'name': 'Stremio', 'package': 'com.stremio.one', 'installed': True},
                                   {'id': 'youtube', 'name': 'YouTube', 'package': 'com.google.android.youtube.tv', 'installed': True}]},
            '/api/recs': {'continue': [], 'picks': []}, '/api/continue': {'items': []}, '/api/who': {'who': None},
            '/api/screens': {'screens': []}, '/api/settings': {'settings': {}}, '/api/sleep': {'ends': None},
            '/api/stremio/discover': {'series': SERIES, 'movies': MOVIES},
            '/api/stremio/library': {'connected': True, 'items': [dict(SERIES[2], inProgress=True, progress=.42), dict(MOVIES[3], inProgress=False)]},
            '/api/stremio/search': {'items': [dict(SERIES[0], title='Bluey'), dict(MOVIES[0], title='Bluey: Sample film')]},
            '/__fixture': STATE,
        }
        if path == '/api/events':
            self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.send_header('Cache-Control', 'no-cache'); self.end_headers()
            try:
                for _ in range(2400):
                    self.wfile.write(('event: now\ndata: ' + json.dumps(self.now()) + '\n\n').encode()); self.wfile.flush(); time.sleep(.25)
            except (BrokenPipeError, ConnectionResetError): pass
            return
        if path == '/api/youtube/search':
            query = parse_qs(url.query).get('q', [''])[0]
            if query == 'error': return self.reply({'ok': False, 'error': 'YouTube unavailable'}, 502)
            if query == 'slow': time.sleep(1.5)
            return self.reply({'ok': True, 'items': [] if query == 'empty' else [
                {'id': 'abcdefghijk', 'title': query + ' video', 'channel': 'A photography channel', 'duration': '12:34', 'thumbnail': 'https://i.ytimg.com/vi/abcdefghijk/mqdefault.jpg'},
                {'id': 'abc_DEF-123', 'title': 'A very long video title with more words than fit in a single line on a phone screen', 'channel': 'Another channel', 'duration': '3:21', 'thumbnail': 'https://i.ytimg.com/vi/abc_DEF-123/mqdefault.jpg'}]})
        if path in ('/api/youtube/account', '/api/youtube/link'): return self.reply({'ok': True, **self.youtube_account()})
        if path == '/api/youtube/library':
            account = self.youtube_account()
            if not account['connected']: return self.reply({'ok': False, 'error': 'Connect YouTube on this phone first.'}, 409)
            if STATE['youtube_library_error']: return self.reply({'ok': False, 'error': 'Could not load your YouTube library.'}, 502)
            q = parse_qs(url.query); kind = q.get('kind', [''])[0]; name = account['account']['title']
            if kind in ('subscriptions', 'playlists'):
                items = [{'id': 'UC' + 'a'*22 if kind == 'subscriptions' else 'PL' + 'a'*20, 'title': name + (' channel' if kind == 'subscriptions' else ' playlist'), 'thumbnail': '', 'count': 12}]
            else:
                second = q.get('page') == ['page-2']
                video_id = 'aqz-KE-bpKQ' if second else 'jNQXAC9IVRw'
                items = [{'id': video_id, 'title': name + (' second video' if second else ' liked video' if kind == 'liked' else ' selected video'), 'channel': name, 'thumbnail': 'https://i.ytimg.com/vi/' + video_id + '/mqdefault.jpg', 'duration': '0:19'}]
            return self.reply({'ok': True, 'items': [] if STATE['youtube_library_empty'] else items,
                               'kind': kind if kind in ('subscriptions', 'playlists') else 'videos',
                               'next': 'page-2' if kind == 'liked' and 'page' not in q and not STATE['youtube_library_empty'] else None})
        if path in routes: return self.reply({'ok': True, **routes[path]})
        if path.startswith('/api/'): return self.reply({'ok': False, 'error': 'No fixture endpoint'}, 404)
        return super().do_GET()
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get('Content-Length', '0'))) or b'{}')
        path = urlsplit(self.path).path
        if path == '/__fixture':
            for key in ('active', 'home_request', 'open_request', 'fail_switch', 'accounts', 'youtube_configured', 'youtube_library_error', 'youtube_library_empty', 'stremio_detail_error', 'stremio_sources_error', 'stremio_sources_empty', 'stremio_play_error', 'stremio_provider_failed'):
                if key in body: STATE[key] = body[key]
            if body.get('clear_events'): STATE['events'] = []
            return self.reply({'ok': True})
        STATE['events'].append({'path': path, 'body': body, 'platform': STATE['active']})
        if path == '/api/stremio/play':
            if STATE.get('stremio_play_error'): return self.reply({'ok': False, 'error': 'Stremio could not open that source.'}, 502)
            choice = STATE.get('stremio_choices', {}).get(body.get('id'))
            if not choice: return self.reply({'ok': False, 'error': 'Choose a source again.'}, 409)
            STATE['events'][-1]['choice'] = choice
            return self.reply({'ok': True})
        if path == '/api/youtube/link':
            STATE['accounts'][self.youtube_client()] = {'state': 'pending', 'link': {'user_code': 'WWWWWWWWWWWWWWW', 'verification_url': 'https://www.google.com/device', 'expires_in': 1800, 'interval': 5}}
            return self.reply({'ok': True, **self.youtube_account()})
        if path in ('/api/youtube/disconnect', '/api/youtube/link/cancel'):
            STATE['accounts'].pop(self.youtube_client(), None)
            return self.reply({'ok': True, **self.youtube_account()})
        if path == '/api/key' and body.get('key') == 'home': STATE['home_request'] += 1; STATE['open_request'] = None
        if path == '/api/platform/apple/opened' and (STATE['open_request'] or {}).get('serial') == body.get('serial'): STATE['open_request'] = None
        if path == '/api/platform':
            time.sleep(.8)
            if STATE['fail_switch']:
                STATE['fail_switch'] = False
                return self.reply({'ok': False, 'error': 'Apple TV is unavailable. The current input has been kept.'}, 503)
            STATE['active'] = body['platform']
            return self.reply({'ok': True, 'platform': platform()})
        if path == '/api/platform/home-code': return self.reply({'ok': True, 'code': '123456', 'expires_in': 180})
        return self.reply({'ok': True})

ThreadingHTTPServer(('127.0.0.1', int(os.environ.get('COUCHTV_FIXTURE_PORT', '8756'))), Handler).serve_forever()
