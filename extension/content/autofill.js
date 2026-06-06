// WPW Autofill Content Script
// Injected on-demand via scripting.executeScript

(function() {
  'use strict';
  
  // Find login form fields
  function isTextInput(el) {
    const type = (el.getAttribute('type') || 'text').toLowerCase();
    return ['text', 'email', 'tel', 'search', 'url', 'number'].includes(type);
  }

  function isUsernameInput(input) {
    if (!isTextInput(input) && input.type !== 'text') return false;
    const ac = input.autocomplete;
    if (ac === 'username' || ac === 'email' || ac === 'nickname') return true;
    const name = (input.name || '').toLowerCase();
    const id = (input.id || '').toLowerCase();
    const patterns = ['username', 'email', 'login', 'user', 'account', 'phone', 'mobile'];
    return patterns.some(p => name.includes(p) || id.includes(p));
  }

  function findUsernameField(container, passwordField) {
    const inputs = container.querySelectorAll('input');
    // Pass 1: find by autocomplete or name/id patterns
    for (const input of inputs) {
      if (input === passwordField) continue;
      if (isUsernameInput(input)) return input;
    }
    // Pass 2: first text input before password field
    for (const input of inputs) {
      if (input === passwordField) break;
      if (input.value && isTextInput(input)) return input;
    }
    // Pass 3: closest text input to password field (by DOM position)
    let closest = null;
    let closestDist = Infinity;
    const pIdx = Array.from(inputs).indexOf(passwordField);
    for (let i = 0; i < inputs.length; i++) {
      if (inputs[i] === passwordField || !isTextInput(inputs[i])) continue;
      const dist = Math.abs(i - pIdx);
      if (dist < closestDist) {
        closestDist = dist;
        closest = inputs[i];
      }
    }
    return closest;
  }

  function findLoginForm() {
    const passwordFields = document.querySelectorAll('input[type="password"]');
    if (passwordFields.length === 0) return null;
    
    const passwordField = passwordFields[0];
    let usernameField = null;
    
    // Look for username field in the same form
    const form = passwordField.closest('form');
    if (form) {
      usernameField = findUsernameField(form, passwordField);
    }
    
    // Fallback: progressive search
    if (!usernameField) {
      const allInputs = document.querySelectorAll('input');
      const passwordRect = passwordField.getBoundingClientRect();
      
      // Pass 1: check autocomplete attribute (most reliable)
      for (const input of allInputs) {
        if (input === passwordField) continue;
        const ac = input.autocomplete;
        if (ac === 'username' || ac === 'email' || ac === 'nickname') {
          usernameField = input;
          break;
        }
      }
      
      // Pass 2: check name/id for common patterns
      if (!usernameField) {
        const patterns = ['username', 'email', 'login', 'user', 'account', 'mail', 'phone', 'mobile', 'name'];
        for (const input of allInputs) {
          if (input === passwordField) continue;
          const name = (input.name || '').toLowerCase();
          const id = (input.id || '').toLowerCase();
          if (patterns.some(p => name.includes(p) || id.includes(p))) {
            usernameField = input;
            break;
          }
        }
      }
      
      // Pass 3: proximity match (up to 300px)
      if (!usernameField) {
        for (const input of allInputs) {
          if (input === passwordField || !isTextInput(input)) continue;
          const rect = input.getBoundingClientRect();
          if (rect.width > 0 && Math.abs(rect.top - passwordRect.top) < 300) {
            usernameField = input;
            break;
          }
        }
      }
      
      // Pass 4: any non-empty text input
      if (!usernameField) {
        for (const input of allInputs) {
          if (input === passwordField || !isTextInput(input)) continue;
          if (input.value && input.offsetParent !== null) {
            usernameField = input;
            break;
          }
        }
      }
      
      // Pass 5: any visible text input
      if (!usernameField) {
        for (const input of allInputs) {
          if (input === passwordField || !isTextInput(input)) continue;
          if (input.offsetParent !== null) {
            usernameField = input;
            break;
          }
        }
      }
      
      // Pass 6: any input with a value (captures fields with non-standard types)
      if (!usernameField) {
        for (const input of allInputs) {
          if (input === passwordField || input.type === 'hidden') continue;
          if (input.value) {
            usernameField = input;
            break;
          }
        }
      }
      
      // Pass 7: direct lookups by common IDs/selectors
      if (!usernameField) {
        usernameField = document.getElementById('username')
          || document.getElementById('email')
          || document.getElementById('login')
          || document.getElementById('user')
          || document.getElementById('account')
          || document.getElementById('phone')
          || document.querySelector('[autocomplete="username"]')
          || document.querySelector('[autocomplete="email"]')
          || document.querySelector('[name="username"]')
          || document.querySelector('[name="email"]')
          || document.querySelector('[name="login"]');
      }
    }
    
    return { usernameField, passwordField, form: passwordField.closest('form') };
  }
  
  // Fill a field with proper event dispatching
  function fillField(field, value) {
    if (!field || !value) return;
    
    field.focus();
    
    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype, 'value'
    ).set;
    nativeInputValueSetter.call(field, value);
    
    field.dispatchEvent(new Event('input', { bubbles: true }));
    field.dispatchEvent(new Event('change', { bubbles: true }));
    field.dispatchEvent(new KeyboardEvent('keyup', { bubbles: true }));
  }
  
  function findUsernameDirect() {
    for (const sel of ['#username', '#email', '#login', '#user',
                        '[autocomplete="username"]', '[autocomplete="email"]',
                        '[name="username"]', '[name="email"]', '[name="login"]',
                        'input[type="email"]']) {
      const el = document.querySelector(sel);
      if (el) return el;
    }
    const all = document.querySelectorAll('input');
    for (let i = 0; i < all.length; i++) {
      if (all[i].type === 'password') {
        for (let j = i - 1; j >= 0; j--) {
          if (all[j].type === 'text' || all[j].type === 'email' || !all[j].type) {
            return all[j];
          }
        }
        break;
      }
    }
    return null;
  }

  // Listen for fill messages from the extension
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.type === 'fill') {
      const form = findLoginForm();
      if (form) {
        const uf = findUsernameDirect() || form.usernameField;
        fillField(uf, message.username);
        fillField(form.passwordField, message.password);
        sendResponse({ success: true });
      } else {
        sendResponse({ success: false, error: 'No login form found' });
      }
    }
    
    if (message.type === 'detectForm') {
      const form = findLoginForm();
      sendResponse({ found: !!form });
    }

    if (message.type === 'captureCredentials') {
      const form = findLoginForm();
      // Debug: check direct lookup
      const directUsername = document.getElementById('username');
      const allInputCount = document.querySelectorAll('input').length;
      if (form && form.passwordField && form.passwordField.value) {
        sendResponse({
          found: true,
          username: form.usernameField ? form.usernameField.value : '',
          password: form.passwordField.value,
          title: document.title,
          _debug: {
            hasUsernameField: !!form.usernameField,
            usernameFieldId: form.usernameField ? form.usernameField.id : null,
            directUsernameVal: directUsername ? directUsername.value : null,
            allInputCount: allInputCount,
            formExists: !!form.form,
          },
        });
      } else {
        sendResponse({ found: false });
      }
    }
  });
  
  // Detect form submission to capture credentials
  document.addEventListener('submit', (e) => {
    const form = e.target;
    const passwordInput = form.querySelector('input[type="password"]');
    if (!passwordInput || !passwordInput.value) return;
    
    const usernameInput = findUsernameField(form, passwordInput);
    
    chrome.runtime.sendMessage({
      type: 'credentialsDetected',
      payload: {
        username: usernameInput ? usernameInput.value : '',
        password: passwordInput.value,
        url: window.location.href,
        title: document.title,
      }
    });
  }, true);
  
  // Notify background that we detected a login form
  const form = findLoginForm();
  if (form) {
    chrome.runtime.sendMessage({
      type: 'formDetected',
      url: window.location.href
    });
  }
})();
