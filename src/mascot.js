const tauri = window.__TAURI__;
if (!tauri) {
  console.error('Tauri API is not available.');
  throw new Error('Tauri API is not available');
}

const { invoke } = tauri.core;
const { listen } = tauri.event;
const { getCurrentWindow } = tauri.window;

const characterImg = document.getElementById('character-img');
const stage = document.getElementById('stage');
const bubble = document.getElementById('speech-bubble');
const inputEl = document.getElementById('input');
const sendBtn = document.getElementById('send');

let speaking = false;
let sending = false;
let fadeTimer = null;

// --- キャラ画像切替（口パク） ---
function setSpeaking(val) {
  speaking = val;
  characterImg.src = val ? 'assets/character/02.png' : 'assets/character/01.png';
}

// --- 吹き出し制御 ---
function showBubble() {
  clearTimeout(fadeTimer);
  bubble.classList.remove('hidden');
  bubble.classList.add('visible');
}

function hideBubble() {
  bubble.classList.remove('visible');
  bubble.classList.add('hidden');
}

function scheduleFade(ms) {
  clearTimeout(fadeTimer);
  fadeTimer = setTimeout(hideBubble, ms);
}

// --- 送信処理 ---
function finishSending() {
  sending = false;
  sendBtn.disabled = false;
  inputEl.focus();
}

async function handleSend() {
  const text = inputEl.value.trim();
  if (!text || sending) return;

  sending = true;
  sendBtn.disabled = true;
  inputEl.value = '';
  inputEl.style.height = 'auto';

  // 吹き出しをローディング表示
  bubble.textContent = '';
  bubble.classList.add('loading');
  showBubble();

  try {
    await invoke('send_message', { message: text });
  } catch (e) {
    bubble.classList.remove('loading');
    bubble.textContent = 'エラー: ' + e;
    scheduleFade(5000);
    finishSending();
  }
}

// --- ストリーミング受信 ---
listen('chat-token', (event) => {
  if (bubble.classList.contains('loading')) {
    bubble.classList.remove('loading');
    bubble.textContent = '';
  }
  bubble.textContent += event.payload;
  bubble.scrollTop = bubble.scrollHeight;
  showBubble();
  if (!speaking) setSpeaking(true);
});

listen('chat-complete', () => {
  bubble.classList.remove('loading');
  setSpeaking(false);
  finishSending();
  scheduleFade(8000);
});

listen('chat-error', (event) => {
  bubble.classList.remove('loading');
  bubble.textContent += '\n[エラー: ' + event.payload + ']';
  setSpeaking(false);
  finishSending();
  scheduleFade(5000);
});

// --- 入力タブ toggle ---
const inputArea = document.getElementById('input-area');

characterImg.addEventListener('click', () => {
  inputArea.classList.toggle('open');
  if (inputArea.classList.contains('open')) {
    inputEl.focus();
  }
});

// 入力エリア内のクリックがキャラに伝播しないようにする
inputArea.addEventListener('click', (e) => e.stopPropagation());

// --- 入力UI ---
sendBtn.addEventListener('click', handleSend);

inputEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
});

inputEl.addEventListener('input', () => {
  inputEl.style.height = 'auto';
  inputEl.style.height = Math.min(inputEl.scrollHeight, 80) + 'px';
});

// --- 右クリックでドラッグ移動 ---
stage.addEventListener('mousedown', async (e) => {
  if (e.button === 2) {
    e.preventDefault();
    await getCurrentWindow().startDragging();
  }
});

stage.addEventListener('contextmenu', (e) => e.preventDefault());
