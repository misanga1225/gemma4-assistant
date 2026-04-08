const tauri = window.__TAURI__;
if (!tauri) {
  console.error('Tauri API is not available. Set app.withGlobalTauri=true or migrate to module imports.');
  throw new Error('Tauri API is not available');
}

const { invoke } = tauri.core;
const { listen } = tauri.event;
const { getCurrentWindow } = tauri.window;
const { WebviewWindow } = tauri.webviewWindow;

const characterImg = document.getElementById('character-img');
const container = document.getElementById('mascot-container');
let speaking = false;

function setSpeaking(val) {
  speaking = val;
  characterImg.src = val ? 'assets/character/02.png' : 'assets/character/01.png';
}

// クリックでchatウィンドウをtoggle
container.addEventListener('click', async () => {
  const chatWindow = await WebviewWindow.getByLabel('chat');
  if (chatWindow) {
    const visible = await chatWindow.isVisible();
    if (visible) {
      await chatWindow.hide();
    } else {
      await chatWindow.show();
      await chatWindow.setFocus();
    }
  }
});

// 右クリックでドラッグ移動
container.addEventListener('mousedown', async (e) => {
  if (e.button === 2) {
    e.preventDefault();
    await getCurrentWindow().startDragging();
  }
});

// 右クリックメニュー無効化
container.addEventListener('contextmenu', (e) => e.preventDefault());

// ストリーミング状態で口パク切り替え
listen('chat-token', () => {
  if (!speaking) setSpeaking(true);
});

listen('chat-complete', () => {
  setSpeaking(false);
});

listen('chat-error', () => {
  setSpeaking(false);
});
