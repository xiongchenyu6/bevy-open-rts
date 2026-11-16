const card = document.querySelector('#install-card');
const button = document.querySelector('#install-button');
const dialog = document.querySelector('#install-dialog');
const standalone = matchMedia('(display-mode: standalone)');
const ios = /iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
const mobile = matchMedia('(pointer: coarse)').matches || /Android|iPhone|iPad/i.test(navigator.userAgent);
let promptEvent;
let installed = false;
let language = 'en';
const copy = {
  en: {
    title: 'Add to your home screen', button: 'Add to home screen', native: 'Install game', close: 'Got it',
    summary: 'More room for the battlefield. Launch from your home screen for the app experience.',
    note: 'The first launch needs an internet connection. Adding a shortcut may still open a browser on some devices.',
    ios: ['Open this page in Safari.', 'Tap Share, then “Add to Home Screen”.', 'Keep “Open as Web App” enabled if shown. Tap Add, then launch the new icon.'],
    android: ['Open your browser menu (⋮ or ☰).', 'Choose “Install app” or “Add to Home screen”.', 'Confirm, then launch Open RTS from the new home screen icon.'],
    desktop: ['Open your browser menu.', 'Choose “Install Open RTS” or “Install this site as an app”.', 'Launch the game from the installed app.'],
  },
  'zh-CN': {
    title: '添加到桌面', button: '添加到桌面', native: '安装游戏', close: '知道了',
    summary: '给战场更多空间。添加到桌面后，从游戏图标启动。',
    note: '首次启动需要联网。部分设备添加的是快捷方式，仍可能显示浏览器栏。',
    ios: ['使用 Safari 打开当前页面。', '点击分享按钮，再选择“添加到主屏幕”。', '如有“作为网页 App 打开”，请保持开启。点击添加，再从桌面图标启动。'],
    android: ['打开浏览器菜单（⋮ 或 ☰）。', '选择“安装应用”或“添加到主屏幕／桌面”。', '确认添加，再从桌面的游戏图标启动。'],
    desktop: ['打开浏览器菜单。', '选择“安装 Open RTS”或“将此站点作为应用安装”。', '从安装后的游戏图标启动。'],
  },
};
function render() {
  const text = copy[language];
  card.setAttribute('aria-label', text.title);
  card.hidden = installed || standalone.matches || navigator.standalone === true || (!mobile && !promptEvent);
  document.querySelector('#install-summary').textContent = text.summary;
  document.querySelector('#install-label').textContent = promptEvent ? text.native : text.button;
  document.querySelector('#install-title').textContent = text.title;
  document.querySelector('#install-close').textContent = text.close;
  document.querySelector('#install-note').textContent = text.note;
  const steps = text[ios ? 'ios' : mobile ? 'android' : 'desktop'];
  document.querySelector('#install-steps').replaceChildren(...steps.map(value => {
    const li = document.createElement('li'); li.textContent = value; return li;
  }));
  if (card.hidden && dialog.open) dialog.close();
}
export function setInstallLanguage(value) { language = value === 'zh-CN' ? value : 'en'; render(); }
window.addEventListener('beforeinstallprompt', event => {
  event.preventDefault();
  if (!ios) promptEvent = event;
  render();
});
window.addEventListener('appinstalled', () => { installed = true; promptEvent = undefined; render(); });
standalone.addEventListener('change', render);
button.addEventListener('click', async () => {
  if (!promptEvent) { dialog.showModal(); return; }
  const event = promptEvent; promptEvent = undefined; button.disabled = true;
  try { await event.prompt(); await event.userChoice; }
  catch { dialog.showModal(); }
  finally { button.disabled = false; render(); }
});
document.querySelector('#install-close').addEventListener('click', () => dialog.close());
render();
