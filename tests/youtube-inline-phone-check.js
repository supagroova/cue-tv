async (page) => {
  const origin = 'http://127.0.0.1:5400';
  const context = await page.context().browser().newContext({ viewport: { width: 393, height: 852 } });
  const phone = await context.newPage();
  const errors = [], checks = [];
  try {
    await phone.addInitScript(() => {
      window.feedFrames = [];
      window.testFeeds = { selected: '11111111-1111-4111-8111-111111111111', profiles: [{id:'11111111-1111-4111-8111-111111111111',name:'Rupert'}, {id:'22222222-2222-4222-8222-222222222222',name:'Ursula'}] };
      window.managedFeeds = 0;
      window.couchNative = { canInlineYouTube: true, canBrowseYouTube: true, canManageYouTubeFeeds: true,
        openYouTubeHome() {}, setYouTubeFeed: (value) => window.feedFrames.push(value),
        getYouTubeFeeds: async () => window.testFeeds,
        selectYouTubeFeed: (id) => { window.testFeeds = {...window.testFeeds, selected:id}; window.dispatchEvent(new CustomEvent('tv:youtube-feeds',{detail:window.testFeeds})); },
        manageYouTubeFeeds: () => { window.managedFeeds++; } };
    });
    phone.on('pageerror', (error) => errors.push(error.message));
    await phone.request.post(origin + '/__fixture', { data: { active: 'apple', accounts: {}, clear_events: true } });
    await phone.goto(origin);
    await phone.getByRole('button', { name: 'Watch', exact: true }).click();
    await phone.waitForFunction(() => { const p = document.querySelector('.pager'); return p.scrollLeft > p.clientWidth * .99; });
    if (await phone.locator('.watch .apps').count()) throw Error('App shortcuts remain in Stremio Watch');
    await phone.getByRole('group', { name: 'Search platform' }).getByRole('button', { name: 'YouTube', exact: true }).click();
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible);
    if (await phone.locator('.watch .apps').count()) throw Error('App shortcuts remain in YouTube Watch');
    if (await phone.getByText('What would you like to watch?', { exact: true }).count()) throw Error('Old empty search message competes with the feed');
    checks.push('Both Watch panels omit the Apps row; native YouTube Home replaces the empty search area');
    const feeds = phone.getByRole('combobox', { name: 'YouTube feed', exact: true });
    await feeds.selectOption('22222222-2222-4222-8222-222222222222');
    await phone.waitForFunction(() => document.querySelector('select[aria-label="YouTube feed"]')?.value === '22222222-2222-4222-8222-222222222222');
    if (await feeds.inputValue() !== '22222222-2222-4222-8222-222222222222') throw Error('Selected feed did not follow native state');
    checks.push('Compact Home feed selector switches between the phone’s Rupert and Ursula feeds');
    const initial = await phone.evaluate(() => window.feedFrames.at(-1));
    const settings = phone.getByRole('button', { name: 'YouTube settings', exact: true });
    await settings.click();
    await phone.waitForFunction((y) => !window.feedFrames.at(-1)?.visible || window.feedFrames.at(-1).frame.y > y, initial.frame.y);
    await settings.click();
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible);
    checks.push('Native feed follows the page when Settings expands and closes');
    await settings.click();
    await phone.getByRole('button', { name: 'Manage YouTube feeds', exact: true }).click();
    if (await phone.evaluate(() => window.managedFeeds) !== 1) throw Error('Native feed management did not open');
    if (await settings.getAttribute('aria-expanded') !== 'false') throw Error('Settings stayed open over feed management');
    const search = phone.getByRole('searchbox', { name: 'Search YouTube', exact: true });
    await search.fill('Melbourne');
    await phone.getByRole('button', { name: 'Watch Melbourne video', exact: true }).waitFor();
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible === false);
    await phone.getByRole('button', { name: 'Clear YouTube search', exact: true }).click();
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible);
    checks.push('Typing shows existing search results and hides the feed; clearing restores Home');
    await phone.getByRole('group', { name: 'Search platform' }).getByRole('button', { name: 'Stremio', exact: true }).click();
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible === false);
    await phone.getByRole('group', { name: 'Search platform' }).getByRole('button', { name: 'YouTube', exact: true }).click();
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible);
    await phone.evaluate(() => window.dispatchEvent(new Event('tv:youtube-feed-opened')));
    await phone.getByRole('button', { name: 'Home', exact: true }).waitFor({ state: 'visible' });
    await phone.waitForFunction(() => window.feedFrames.at(-1)?.visible === false);
    checks.push('Feed hides outside YouTube Watch, and an acknowledged native video returns to Remote');
    for (const value of await phone.evaluate(() => window.feedFrames.filter((v) => v.visible))) {
      if (value.clip.x < 0 || value.clip.y < 0 || value.clip.x + value.clip.width > value.viewport.width + 1 || value.clip.y + value.clip.height > value.viewport.height + 1) throw Error('Feed escaped its viewport');
    }
    if (errors.length) throw Error(errors.join('\n'));
    return { checks, errors };
  } finally { await context.close(); }
}
