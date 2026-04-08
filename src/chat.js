const tauri = window.__TAURI__;
if (!tauri) {
  console.error('Tauri API is not available. Set app.withGlobalTauri=true or migrate to module imports.');
  throw new Error('Tauri API is not available');
}

const { invoke } = tauri.core;
const { listen } = tauri.event;
const { getCurrentWindow } = tauri.window;

const messagesEl = document.getElementById('messages');
const inputEl = document.getElementById('input');
const sendBtn = document.getElementById('send');
const statusEl = document.getElementById('status');
const closeBtn = document.getElementById('close-btn');

let sending = false;
let currentAssistantEl = null;

function finishSending() {
  sending = false;
  sendBtn.disabled = false;
  inputEl.focus();
}

// 閉じるボタン → ウィンドウを非表示（destroyしない）
closeBtn.addEventListener('click', async () => {
  await getCurrentWindow().hide();
});

async function checkConnection() {
  try {
    await invoke('ping_ollama');
    statusEl.textContent = '接続済み';
    statusEl.className = 'connected';
  } catch (e) {
    statusEl.textContent = '未接続';
    statusEl.className = 'error';
  }
}

function addMessage(role, text) {
  const div = document.createElement('div');
  div.className = `message ${role}`;
  div.textContent = text;
  messagesEl.appendChild(div);
  messagesEl.scrollTop = messagesEl.scrollHeight;
  return div;
}

function scrollToBottom() {
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

async function handleSend() {
  const text = inputEl.value.trim();
  if (!text || sending) return;

  sending = true;
  sendBtn.disabled = true;
  inputEl.value = '';
  inputEl.style.height = 'auto';

  addMessage('user', text);
  currentAssistantEl = addMessage('assistant', '');
  currentAssistantEl.classList.add('loading');

  try {
    await invoke('send_message', { message: text });
  } catch (e) {
    if (currentAssistantEl) {
      currentAssistantEl.textContent = 'エラー: ' + e;
      currentAssistantEl.classList.remove('loading');
      currentAssistantEl = null;
    }
    finishSending();
  } finally {
    // 正常時の終了処理は chat-complete / chat-error イベントで行う
  }
}

listen('chat-token', (event) => {
  if (currentAssistantEl) {
    if (currentAssistantEl.classList.contains('loading')) {
      currentAssistantEl.classList.remove('loading');
      currentAssistantEl.textContent = '';
    }
    currentAssistantEl.textContent += event.payload;
    scrollToBottom();
  }
});

listen('chat-error', (event) => {
  if (currentAssistantEl) {
    currentAssistantEl.textContent += '\n[エラー: ' + event.payload + ']';
    currentAssistantEl.classList.remove('loading');
    currentAssistantEl = null;
  }
  finishSending();
});

listen('chat-complete', () => {
  if (currentAssistantEl) {
    currentAssistantEl.classList.remove('loading');
    currentAssistantEl = null;
  }
  finishSending();
});

sendBtn.addEventListener('click', handleSend);

inputEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
});

inputEl.addEventListener('input', () => {
  inputEl.style.height = 'auto';
  inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
});

async function loadConfig() {
  try {
    const config = await invoke('get_config');
    document.getElementById('chat-title').textContent = config.name;
  } catch (e) {
    // デフォルトのまま
  }
}

checkConnection();
loadConfig();
