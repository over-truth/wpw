// WPW Native Messaging Host name
const HOST_NAME = 'com.wpw.host';
const CONNECTION_TIMEOUT = 5000;
const HEARTBEAT_INTERVAL = 30000;

let port = null;
let connectionState = 'disconnected'; // disconnected | connecting | connected
let locked = true;
let vaultExists = false;
let pendingRequests = new Map();
let requestIdCounter = 0;

// Badge management
function updateBadge() {
  if (!vaultExists) {
    chrome.action.setBadgeText({ text: '' });
    chrome.action.setBadgeBackgroundColor({ color: '#808080' });
  } else if (locked) {
    chrome.action.setBadgeText({ text: '!' });
    chrome.action.setBadgeBackgroundColor({ color: '#FF0000' });
  } else {
    chrome.action.setBadgeText({ text: '·' });
    chrome.action.setBadgeBackgroundColor({ color: '#4A90D9' });
  }
}

function updateBadgeWithCount(count) {
  if (!locked && count > 0) {
    chrome.action.setBadgeText({ text: String(count) });
    chrome.action.setBadgeBackgroundColor({ color: '#4A90D9' });
  }
}

// Native Messaging connection
function connectToHost() {
  if (connectionState === 'connecting') return;
  
  connectionState = 'connecting';
  
  try {
    port = chrome.runtime.connectNative(HOST_NAME);
    
    port.onMessage.addListener((message) => {
      handleHostMessage(message);
    });
    
    port.onDisconnect.addListener(() => {
      const error = chrome.runtime.lastError;
      console.log('Host disconnected:', error?.message || 'unknown');
      port = null;
      connectionState = 'disconnected';
      locked = true;
      updateBadge();
      
      // Reject all pending requests
      for (const [id, { reject, timer }] of pendingRequests) {
        clearTimeout(timer);
        reject(new Error('Host disconnected'));
      }
      pendingRequests.clear();
    });
    
    connectionState = 'connected';
    
    // Check initial status
    sendRequest({ type: 'status' }).then(response => {
      if (response.success) {
        locked = response.payload.locked;
        vaultExists = response.payload.vault_exists;
        updateBadge();
      }
    }).catch(() => {
      // Status check failed, but connection is alive
    });
    
  } catch (e) {
    console.error('Failed to connect to host:', e);
    connectionState = 'disconnected';
    locked = true;
    updateBadge();
  }
}

function sendRequest(payload) {
  return new Promise((resolve, reject) => {
    if (!port || connectionState !== 'connected') {
      connectToHost();
      if (!port) {
        reject(new Error('Not connected to host'));
        return;
      }
    }
    
    const id = String(++requestIdCounter);
    const message = { id, ...payload };
    
    const timer = setTimeout(() => {
      pendingRequests.delete(id);
      reject(new Error('Request timeout'));
    }, CONNECTION_TIMEOUT);
    
    pendingRequests.set(id, { resolve, reject, timer });
    
    try {
      port.postMessage(message);
    } catch (e) {
      pendingRequests.delete(id);
      clearTimeout(timer);
      reject(e);
    }
  });
}

function handleHostMessage(message) {
  if (message.type === 'response' && message.id) {
    const pending = pendingRequests.get(message.id);
    if (pending) {
      clearTimeout(pending.timer);
      pendingRequests.delete(message.id);
      pending.resolve(message);
    }
  } else if (message.type === 'event') {
    handleEvent(message.payload);
  }
}

function handleEvent(event) {
  if (event.event === 'locked') {
    locked = true;
    updateBadge();
  } else if (event.event === 'unlocked') {
    locked = false;
    updateBadge();
  }
}

// Tab monitoring
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' && tab.url) {
    // Inject content script for credential detection (always, even when locked)
    if (tab.url.startsWith('http://') || tab.url.startsWith('https://')) {
      chrome.scripting.executeScript({
        target: { tabId },
        files: ['content/autofill.js'],
      }).catch(() => {});
    }

    if (!locked) {
      queryEntriesForTab(tab.url).then(entries => {
        updateBadgeWithCount(entries.length);
        chrome.storage.session.set({ [`tab_${tabId}`]: entries });
      }).catch(() => {});
    }
  }
});

async function queryEntriesForTab(url) {
  try {
    const response = await sendRequest({ type: 'query', payload: { url } });
    if (response.success) {
      return response.payload?.entries || [];
    }
  } catch (e) {
    console.error('Query failed:', e);
  }
  return [];
}

// Message handling from popup and content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'getStatus') {
    sendResponse({
      locked,
      vaultExists,
      connectionState
    });
    return false;
  }
  
  if (message.type === 'unlock') {
    sendRequest({ type: 'unlock', payload: { master_password: message.password } })
      .then(response => {
        if (response.success) {
          locked = false;
          updateBadge();
        }
        sendResponse(response);
      })
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true; // async response
  }
  
  if (message.type === 'lock') {
    sendRequest({ type: 'lock' })
      .then(response => {
        if (response.success) {
          locked = true;
          updateBadge();
        }
        sendResponse(response);
      })
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'query') {
    sendRequest({ type: 'query', payload: { url: message.url } })
      .then(sendResponse)
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'getEntry') {
    sendRequest({ type: 'get_entry', payload: { entry_id: message.entryId } })
      .then(sendResponse)
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'getTotp') {
    sendRequest({ type: 'get_totp', payload: { entry_id: message.entryId } })
      .then(sendResponse)
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'addEntry') {
    sendRequest({ type: 'add_entry', payload: message.entry })
      .then(sendResponse)
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'deleteEntry') {
    sendRequest({ type: 'delete_entry', payload: { entry_id: message.entryId } })
      .then(sendResponse)
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'fillCredentials') {
    // Forward to content script
    chrome.tabs.sendMessage(message.tabId, {
      type: 'fill',
      username: message.username,
      password: message.password
    });
    sendResponse({ success: true });
    return false;
  }
  
  if (message.type === 'formDetected') {
    // Content script reports a login form. We only refresh the badge count here so the
    // user knows credentials are available — we deliberately do NOT auto-fill, even on a
    // single match, because lax subdomain matching could otherwise leak credentials into
    // a sibling subdomain. Auto-fill requires an explicit click in the popup.
    if (!locked && sender.tab?.url) {
      queryEntriesForTab(sender.tab.url).then(entries => {
        updateBadgeWithCount(entries.length);
        if (sender.tab.id != null) {
          chrome.storage.session.set({ [`tab_${sender.tab.id}`]: entries });
        }
      }).catch(() => {});
    }
    sendResponse({ success: true });
    return false;
  }
  
  if (message.type === 'credentialsDetected') {
    // Content script captured credentials from form submission,
    // or popup captured manually (then message.tabId is provided)
    const tabId = sender.tab?.id || message.tabId;
    if (tabId && message.payload) {
      chrome.storage.session.set({ [`detected_${tabId}`]: message.payload }).then(() => {
        sendResponse({ success: true });
      });
    } else {
      sendResponse({ success: true });
    }
    return true;
  }
  
  if (message.type === 'getDetectedCredentials') {
    // Popup requests detected credentials for a tab
    const tabId = message.tabId;
    const key = `detected_${tabId}`;
    chrome.storage.session.get(key).then(result => {
      sendResponse(result[key] || null);
    });
    return true;
  }
  
  if (message.type === 'saveDetectedCredentials') {
    // Popup wants to save detected credentials
    sendRequest({ type: 'add_entry', payload: message.entry })
      .then(response => {
        if (response.success && message.tabId) {
          // Clear detected credentials after saving
          chrome.storage.session.remove(`detected_${message.tabId}`);
          updateBadge();
        }
        sendResponse(response);
      })
      .catch(e => sendResponse({ success: false, error: { code: 'connection_error', message: e.message } }));
    return true;
  }
  
  if (message.type === 'dismissDetectedCredentials') {
    if (message.tabId) {
      chrome.storage.session.remove(`detected_${message.tabId}`);
      updateBadge();
    }
    sendResponse({ success: true });
    return false;
  }
});

// Initialize connection on startup
connectToHost();

// Heartbeat to keep connection alive
setInterval(() => {
  if (port && connectionState === 'connected') {
    sendRequest({ type: 'status' }).catch(() => {
      // Connection lost, will reconnect on next request
    });
  }
}, HEARTBEAT_INTERVAL);

console.log('WPW service worker initialized');
