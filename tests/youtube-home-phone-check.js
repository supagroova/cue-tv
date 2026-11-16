async (page) => {
  const origin = 'http://127.0.0.1:8756';
  const checks = [];
  const context = await page.context().browser().newContext({ viewport: { width: 393, height: 852 } });
  const phone = await context.newPage();
  try {
    await phone.addInitScript(() => {
      window.youtubeHomeOpened = 0;
      window.couchNative = { canBrowseYouTube: true, openYouTubeHome: () => { window.youtubeHomeOpened += 1; } };
    });
    await phone.request.post(origin + '/__fixture', { data: { active: 'android', accounts: {}, youtube_configured: true, clear_events: true } });
    await phone.goto(origin);
    await phone.getByRole('button', { name: 'Watch', exact: true }).click();
    await phone.waitForFunction(() => { const p = document.querySelector('.pager'); return !document.querySelector('.watch-page')?.inert && (!p || p.scrollLeft > p.clientWidth * .99); });
    await phone.getByRole('group', { name: 'Search platform' }).getByRole('button', { name: 'YouTube', exact: true }).click();
    await phone.getByRole('searchbox', { name: 'Search YouTube' }).fill('Melbourne');
    await phone.getByRole('button', { name: 'Watch Melbourne video', exact: true }).waitFor();
    await phone.getByRole('button', { name: 'Open YouTube Home', exact: true }).click();
    if (await phone.evaluate(() => window.youtubeHomeOpened) !== 1) throw new Error('Native Home browser was not requested exactly once');
    if (await phone.getByRole('searchbox', { name: 'Search YouTube' }).inputValue() !== 'Melbourne') throw new Error('Home browsing lost the existing search');
    const state = await (await phone.request.get(origin + '/__fixture')).json();
    if ((state.events || []).length) throw new Error('Opening the phone feed changed the TV');
    checks.push('Native Home opens through the trusted bridge without changing the TV or losing search');
    if (await phone.getByRole('region', { name: 'YouTube account', exact: true }).count()) throw new Error('Account panel is prominent before settings are opened');
    const homeSize = await phone.getByRole('button', { name: 'Open YouTube Home', exact: true }).boundingBox();
    if (!homeSize || homeSize.height < 44 || homeSize.height > 52) throw new Error('Home shortcut is not compact and tappable');
    await phone.screenshot({ path: 'output/playwright/youtube-native-home-entry.png' });
    const settings = phone.getByRole('button', { name: 'YouTube settings', exact: true });
    await settings.click();
    const privacy = phone.getByRole('link', { name: 'YouTube privacy', exact: true });
    if (await privacy.getAttribute('href') !== '/youtube/privacy.html') throw new Error('Privacy notice missing');
    await phone.getByRole('region', { name: 'YouTube account', exact: true }).waitFor();
    await phone.screenshot({ path: 'output/playwright/youtube-compact-settings.png' });
    await privacy.focus(); await phone.keyboard.press('Escape');
    if (await settings.getAttribute('aria-expanded') !== 'false' || !await settings.evaluate((el) => el === document.activeElement)) throw new Error('Escape did not close settings and return focus');
    if (await phone.getByRole('searchbox', { name: 'Search YouTube' }).inputValue() !== 'Melbourne') throw new Error('Settings lost the search');
    checks.push('Settings starts collapsed, keeps connection controls and privacy accessible, closes with Escape and restores focus');
    for (const width of [320, 393, 430, 900]) {
      await phone.setViewportSize({ width, height: 852 });
      if (await phone.evaluate(() => document.documentElement.scrollWidth > innerWidth)) throw new Error('Overflow at '+width);
    }
    checks.push('Native Home entry and privacy link fit phone and wide layouts');
    await page.setViewportSize({ width: 393, height: 852 });
    await page.goto(origin);
    await page.getByRole('button', { name: 'Watch', exact: true }).click();
    await page.waitForFunction(() => { const p = document.querySelector('.pager'); return !document.querySelector('.watch-page')?.inert && (!p || p.scrollLeft > p.clientWidth * .99); });
    await page.getByRole('group', { name: 'Search platform' }).getByRole('button', { name: 'YouTube', exact: true }).click();
    const link = page.getByRole('link', { name: 'Open YouTube Home', exact: true });
    if (await link.getAttribute('href') !== 'https://www.youtube.com/') throw new Error('Older apps need the explicit YouTube link');
    checks.push('Web and older native versions have an honest YouTube link instead of an unavailable native action');
    return { checks };
  } finally { await context.close(); }
}
