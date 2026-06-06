// State
let currentView = 'locked';
let currentEntries = [];
let selectedEntry = null;
let passwordVisible = false;
let totpInterval = null;
let currentTabId = null;

// DOM elements
const views = {
  notConnected: document.getElementById('notConnected'),
  noVault: document.getElementById('noVault'),
  locked: document.getElementById('locked'),
  unlocked: document.getElementById('unlocked'),
  entryDetail: document.getElementById('entryDetail'),
};

const elements = {
  lockBtn: document.getElementById('lockBtn'),
  unlockForm: document.getElementById('unlockForm'),
  masterPassword: document.getElementById('masterPassword'),
  unlockError: document.getElementById('unlockError'),
  searchInput: document.getElementById('searchInput'),
  entriesList: document.getElementById('entriesList'),
  noEntries: document.getElementById('noEntries'),
  backBtn: document.getElementById('backBtn'),
  detailTitle: document.getElementById('detailTitle'),
  detailUsername: document.getElementById('detailUsername'),
  detailPassword: document.getElementById('detailPassword'),
  togglePassword: document.getElementById('togglePassword'),
  totpSection: document.getElementById('totpSection'),
  detailTotp: document.getElementById('detailTotp'),
  totpTimer: document.getElementById('totpTimer'),
  fillBtn: document.getElementById('fillBtn'),
  deleteBtn: document.getElementById('deleteBtn'),
  savePassword: document.getElementById('savePassword'),
  saveSite: document.getElementById('saveSite'),
  saveUsername: document.getElementById('saveUsername'),
  savePasswordValue: document.getElementById('savePasswordValue'),
  saveBtn: document.getElementById('saveBtn'),
  dismissBtn: document.getElementById('dismissBtn'),
  captureBtn: document.getElementById('captureBtn'),
  notification: document.getElementById('notification'),
  confirmOverlay: document.getElementById('confirmOverlay'),
  confirmMessage: document.getElementById('confirmMessage'),
  confirmOkBtn: document.getElementById('confirmOkBtn'),
  confirmCancelBtn: document.getElementById('confirmCancelBtn'),
};

// Custom confirm dialog
let confirmResolve = null;

function showConfirm(message) {
  return new Promise(resolve => {
    confirmResolve = resolve;
    elements.confirmMessage.textContent = message;
    elements.confirmOverlay.classList.remove('hidden');
  });
}

function hideConfirm() {
  elements.confirmOverlay.classList.add('hidden');
  confirmResolve = null;
}

elements.confirmOkBtn.addEventListener('click', () => {
  if (confirmResolve) {
    confirmResolve(true);
    hideConfirm();
  }
});

elements.confirmCancelBtn.addEventListener('click', () => {
  if (confirmResolve) {
    confirmResolve(false);
    hideConfirm();
  }
});

elements.confirmOverlay.addEventListener('click', (event) => {
  if (event.target === elements.confirmOverlay && confirmResolve) {
    confirmResolve(false);
    hideConfirm();
  }
});

// Initialize
async function init() {
  try {
    const status = await chrome.runtime.sendMessage({ type: 'getStatus' });
    
    if (status.connectionState !== 'connected') {
      showView('notConnected');
      return;
    }
    
    if (!status.vaultExists) {
      showView('noVault');
      return;
    }
    
    // Get current tab id
    try {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      currentTabId = tab?.id;
    } catch (_) {
      currentTabId = null;
    }
    
    if (status.locked) {
      showView('locked');
      elements.masterPassword.focus();
    } else {
      await loadEntries();
      await checkDetectedCredentials();
      showView('unlocked');
    }
  } catch (_) {
    showView('notConnected');
  }
}

function showView(viewName) {
  currentView = viewName;
  Object.values(views).forEach(v => v.classList.add('hidden'));
  views[viewName].classList.remove('hidden');
  
  const hideActions = viewName === 'locked' || viewName === 'notConnected' || viewName === 'noVault';
  elements.lockBtn.style.display = hideActions ? 'none' : 'block';
  elements.captureBtn.style.display = hideActions ? 'none' : 'block';
}

async function loadEntries() {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab?.url) {
      const response = await chrome.runtime.sendMessage({ type: 'query', url: tab.url });
      if (response.success) {
        currentEntries = response.payload?.entries || [];
        renderEntries(currentEntries);
      }
    }
  } catch (_) {
    // silently fail
  }
}

function renderEntries(entries) {
  elements.entriesList.innerHTML = '';
  elements.noEntries.classList.toggle('hidden', entries.length > 0);
  
  entries.forEach(entry => {
    const item = document.createElement('div');
    item.className = 'entry-item';
    item.innerHTML = `
      <div class="entry-info">
        <div class="entry-title">${escapeHtml(entry.title)}</div>
        <div class="entry-username">${escapeHtml(entry.username || '')}</div>
        <div class="entry-url">${escapeHtml(entry.url || '')}</div>
      </div>
      <button class="entry-delete-btn" title="删除">🗑️</button>
    `;
    item.addEventListener('click', () => selectEntry(entry));
    
    const deleteBtn = item.querySelector('.entry-delete-btn');
    deleteBtn.addEventListener('click', async (event) => {
      event.stopPropagation();
      const confirmed = await showConfirm(`确定删除「${entry.title}」吗？`);
      if (!confirmed) return;
      try {
        const response = await chrome.runtime.sendMessage({ type: 'deleteEntry', entryId: entry.id });
        if (response.success) {
          currentEntries = currentEntries.filter(item => item.id !== entry.id);
          renderEntries(currentEntries);
          showNotification('密码已删除', 'success');
        } else {
          showNotification(response.error?.message || '删除失败', 'error');
        }
      } catch (err) {
        showNotification('连接错误：' + err.message, 'error');
      }
    });
    
    elements.entriesList.appendChild(item);
  });
}

async function selectEntry(entry) {
  selectedEntry = entry;
  passwordVisible = false;
  
  // Get full entry details
  const response = await chrome.runtime.sendMessage({ type: 'getEntry', entryId: entry.id });
  if (response.success) {
    selectedEntry = { ...entry, ...response.payload };
  }
  
  elements.detailTitle.textContent = entry.title;
  elements.detailUsername.textContent = selectedEntry.username || '(无)';
  elements.detailPassword.textContent = '••••••••';
  elements.togglePassword.textContent = '👁';
  
  // TOTP
  if (selectedEntry.totp_secret) {
    elements.totpSection.style.display = 'block';
    updateTotp();
  } else {
    elements.totpSection.style.display = 'none';
  }
  
  showView('entryDetail');
}

async function updateTotp() {
  if (!selectedEntry?.totp_secret) return;
  
  const response = await chrome.runtime.sendMessage({ type: 'getTotp', entryId: selectedEntry.id });
  if (response.success) {
    elements.detailTotp.textContent = response.payload.code;
    elements.totpTimer.textContent = `${response.payload.remaining}秒`;
  }
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str || '';
  return div.innerHTML;
}

// Event listeners
elements.unlockForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const password = elements.masterPassword.value;
  elements.unlockError.classList.add('hidden');
  
  try {
    const response = await chrome.runtime.sendMessage({ type: 'unlock', password });
    
    if (response.success) {
      elements.masterPassword.value = '';
      await loadEntries();
      await checkDetectedCredentials();
      showView('unlocked');
    } else {
      elements.unlockError.textContent = response.error?.message || '解锁失败';
      elements.unlockError.classList.remove('hidden');
      elements.masterPassword.value = '';
      elements.masterPassword.focus();
    }
  } catch (err) {
    elements.unlockError.textContent = '连接错误：' + err.message;
    elements.unlockError.classList.remove('hidden');
  }
});

elements.lockBtn.addEventListener('click', async () => {
  await chrome.runtime.sendMessage({ type: 'lock' });
  showView('locked');
  elements.masterPassword.value = '';
  elements.masterPassword.focus();
});

elements.backBtn.addEventListener('click', () => {
  showView('unlocked');
  if (totpInterval) clearInterval(totpInterval);
});

elements.togglePassword.addEventListener('click', () => {
  passwordVisible = !passwordVisible;
  elements.detailPassword.textContent = passwordVisible
    ? (selectedEntry?.password || '')
    : '••••••••';
  elements.togglePassword.textContent = passwordVisible ? '🙈' : '👁';
});

elements.fillBtn.addEventListener('click', async () => {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  await chrome.runtime.sendMessage({
    type: 'fillCredentials',
    tabId: tab.id,
    username: selectedEntry.username,
    password: selectedEntry.password,
  });
  window.close();
});

elements.deleteBtn.addEventListener('click', async () => {
  if (!selectedEntry) return;
  const confirmed = await showConfirm(`确定删除「${selectedEntry.title}」吗？`);
  if (!confirmed) return;

  try {
    const response = await chrome.runtime.sendMessage({ type: 'deleteEntry', entryId: selectedEntry.id });
    if (response.success) {
      if (totpInterval) clearInterval(totpInterval);
      currentEntries = currentEntries.filter(item => item.id !== selectedEntry.id);
      renderEntries(currentEntries);
      showView('unlocked');
      showNotification('密码已删除', 'success');
    } else {
      showNotification(response.error?.message || '删除失败', 'error');
    }
  } catch (err) {
    showNotification('连接错误：' + err.message, 'error');
  }
});

// Copy buttons
document.querySelectorAll('.copy-btn').forEach(btn => {
  btn.addEventListener('click', async () => {
    const field = btn.dataset.field;
    let text = '';
    if (field === 'username') text = selectedEntry?.username || '';
    else if (field === 'password') text = selectedEntry?.password || '';
    else if (field === 'totp') text = elements.detailTotp.textContent;
    
    if (text) {
      try {
        await navigator.clipboard.writeText(text);
        showNotification('已复制到剪贴板', 'success');
      } catch (_) {}
    }
  });
});

elements.searchInput.addEventListener('input', (event) => {
  const query = event.target.value.toLowerCase();
  const filtered = currentEntries.filter(entry =>
    entry.title.toLowerCase().includes(query) ||
    (entry.username || '').toLowerCase().includes(query) ||
    (entry.url || '').toLowerCase().includes(query)
  );
  renderEntries(filtered);
});

// TOTP refresh interval
setInterval(() => {
  if (currentView === 'entryDetail' && selectedEntry?.totp_secret) {
    updateTotp();
  }
}, 1000);

async function checkDetectedCredentials() {
  if (!currentTabId) return;
  const response = await chrome.runtime.sendMessage({ type: 'getDetectedCredentials', tabId: currentTabId });
  if (response && response.password) {
    elements.saveSite.textContent = response.title || response.url || '';
    elements.saveUsername.textContent = response.username || '(空)';
    elements.savePasswordValue.textContent = '••••••••';
    elements.savePassword.classList.remove('hidden');
  } else {
    elements.savePassword.classList.add('hidden');
  }
}

elements.saveBtn.addEventListener('click', async () => {
  const response = await chrome.runtime.sendMessage({ type: 'getDetectedCredentials', tabId: currentTabId });
  if (!response || !response.password) return;

  let title = response.title;
  if (!title && response.url) {
    try {
      title = new URL(response.url).hostname;
    } catch (_) {
      title = response.url;
    }
  }
  title = title || '未命名';
  const result = await chrome.runtime.sendMessage({
    type: 'saveDetectedCredentials',
    tabId: currentTabId,
    entry: {
      title,
      url: response.url,
      username: response.username || '',
      password: response.password,
    },
  });

  if (result.success) {
    elements.savePassword.classList.add('hidden');
    await loadEntries();
    renderEntries(currentEntries);
  }
});

elements.dismissBtn.addEventListener('click', async () => {
  await chrome.runtime.sendMessage({ type: 'dismissDetectedCredentials', tabId: currentTabId });
  elements.savePassword.classList.add('hidden');
});

function showNotification(text, type) {
  elements.notification.textContent = text;
  elements.notification.className = 'notification ' + type;
  elements.notification.classList.remove('hidden');
  setTimeout(() => {
    elements.notification.classList.add('hidden');
  }, 3000);
}

elements.captureBtn.addEventListener('click', async () => {
  if (!currentTabId) return;

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  const tabUrl = tab?.url || '';

  let response;
  try {
    response = await chrome.tabs.sendMessage(currentTabId, { type: 'captureCredentials' });
  } catch (_) {
    try {
      await chrome.scripting.executeScript({
        target: { tabId: currentTabId },
        files: ['content/autofill.js'],
      });
      response = await chrome.tabs.sendMessage(currentTabId, { type: 'captureCredentials' });
    } catch (err) {
      showNotification('无法访问页面：' + (err.message || '未知错误'), 'error');
      return;
    }
  }

  try {
    if (response && response.found) {
      let username = response.username;
      if (!username && response._debug && response._debug.directUsernameVal) {
        username = response._debug.directUsernameVal;
      }

      await chrome.runtime.sendMessage({
        type: 'credentialsDetected',
        tabId: currentTabId,
        payload: {
          username: username,
          password: response.password,
          url: tabUrl,
          title: response.title,
        },
      });

      setTimeout(async () => {
        await checkDetectedCredentials();
        showNotification('密码已捕获', 'success');
      }, 100);
    } else {
      showNotification('此页面未找到登录表单', 'error');
    }
  } catch (err) {
    showNotification('无法访问页面：' + (err.message || '未知错误'), 'error');
  }
});

// Initialize
init();
