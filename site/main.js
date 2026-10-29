/* Enclave homepage — progressive enhancement only.
   Every view is a self-contained demo that runs in the page; nothing is sent
   anywhere. Each interaction degrades to readable content without JavaScript. */
(function () {
  'use strict';

  var root = document.documentElement;
  var still = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
  var reducedMotion = still ? still.matches : false;
  var THEME_KEY = 'enclave-site-theme';

  /* ---------- Theme ---------- */

  var toggle = document.querySelector('.theme-toggle');

  function readStoredTheme() {
    try {
      var value = localStorage.getItem(THEME_KEY);
      return value === 'light' || value === 'dark' ? value : null;
    } catch (error) {
      return null;
    }
  }

  function applyTheme(theme) {
    root.setAttribute('data-theme', theme);
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta) {
      meta.setAttribute('content', theme === 'light' ? '#f1ede5' : '#16130f');
    }
    if (toggle) {
      var label = 'Switch to ' + (theme === 'light' ? 'dark' : 'light') + ' theme';
      toggle.setAttribute('aria-label', label);
      toggle.setAttribute('title', label);
    }
  }

  if (toggle) {
    applyTheme(root.getAttribute('data-theme') === 'light' ? 'light' : 'dark');
    toggle.addEventListener('click', function () {
      var next = root.getAttribute('data-theme') === 'light' ? 'dark' : 'light';
      try {
        localStorage.setItem(THEME_KEY, next);
      } catch (error) {
        /* The choice still applies for this page view. */
      }
      applyTheme(next);
    });
  }

  if (still && still.addEventListener) {
    still.addEventListener('change', function (event) {
      reducedMotion = event.matches;
      if (!readStoredTheme() && toggle) {
        applyTheme(event.matches ? 'light' : 'dark');
      }
      decideTour();
    });
  }

  /* ---------- Shared demo feedback ---------- */

  var windowStatus = document.querySelector('[data-window-status]');
  var windowStatusText = document.querySelector('[data-window-status-text]');
  var windowStatusTimer = 0;
  var DEFAULT_STATUS = 'Encrypted';

  function setWindowStatus(text, tone) {
    if (!windowStatus || !windowStatusText) return;
    window.clearTimeout(windowStatusTimer);
    windowStatus.classList.toggle('is-local', tone === 'local');
    windowStatus.classList.toggle('is-synced', tone === 'synced');
    windowStatusText.textContent = text;
    if (text !== DEFAULT_STATUS) {
      windowStatusTimer = window.setTimeout(function () {
        windowStatus.classList.remove('is-local', 'is-synced');
        windowStatusText.textContent = DEFAULT_STATUS;
      }, 2200);
    }
  }

  var toast = document.querySelector('[data-toast]');
  var toastTimer = 0;
  function showToast(text) {
    if (!toast) return;
    window.clearTimeout(toastTimer);
    toast.textContent = text;
    toast.classList.add('is-visible');
    toastTimer = window.setTimeout(function () {
      toast.classList.remove('is-visible');
    }, 2200);
  }

  /* ---------- Header ---------- */

  var header = document.querySelector('[data-header]');
  var nav = document.getElementById('site-nav');
  var menu = document.querySelector('[data-menu-toggle]');

  if (header) {
    var stick = function () {
      header.classList.toggle('is-stuck', window.scrollY > 8);
    };
    window.addEventListener('scroll', stick, { passive: true });
    stick();
  }

  function closeMenu(returnFocus) {
    if (!nav || !menu) return;
    nav.classList.remove('is-open');
    menu.setAttribute('aria-expanded', 'false');
    if (returnFocus) menu.focus();
  }

  if (nav && menu) {
    menu.addEventListener('click', function () {
      var open = nav.classList.toggle('is-open');
      menu.setAttribute('aria-expanded', String(open));
      if (open) {
        var first = nav.querySelector('a');
        if (first) first.focus();
      }
    });
    nav.addEventListener('click', function (event) {
      if (event.target.closest('a')) closeMenu(false);
    });
    document.addEventListener('click', function (event) {
      if (!header.contains(event.target)) closeMenu(false);
    });
    document.addEventListener('keydown', function (event) {
      if (event.key === 'Escape' && nav.classList.contains('is-open')) {
        closeMenu(true);
      }
    });
  }

  /* ---------- Reveals ---------- */

  function setupReveals() {
    var items = Array.prototype.slice.call(document.querySelectorAll('[data-reveal]'));
    if (!items.length) return;

    if (reducedMotion || !('IntersectionObserver' in window)) {
      items.forEach(function (item) {
        item.classList.add('is-in');
      });
      return;
    }

    root.classList.add('js');

    var onScreen = function (item) {
      return item.getBoundingClientRect().top < window.innerHeight * 0.94;
    };

    items.forEach(function (item) {
      if (onScreen(item)) item.classList.add('is-in');
    });

    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (!entry.isIntersecting) return;
          entry.target.classList.add('is-in');
          observer.unobserve(entry.target);
        });
      },
      { rootMargin: '0px 0px -8% 0px' }
    );

    items.forEach(function (item) {
      if (!item.classList.contains('is-in')) observer.observe(item);
    });
  }

  /* ---------- Tabs and the scroll tour ---------- */

  var hero = document.querySelector('[data-hero]');
  var track = document.querySelector('[data-track]');
  var strip = document.querySelector('.tour-tabs');
  var tabs = strip ? Array.prototype.slice.call(strip.querySelectorAll('[role="tab"]')) : [];
  var panels = tabs.map(function (tab) {
    return document.getElementById(tab.getAttribute('aria-controls'));
  });
  var appTitle = document.querySelector('[data-app-title]');
  var pin = document.querySelector('.tour-pin');
  var tourCounter = document.querySelector('[data-tour-counter]');
  var tourPrev = document.querySelector('[data-tour-prev]');
  var tourNext = document.querySelector('[data-tour-next]');
  var scrollMedia = window.matchMedia ? window.matchMedia('(min-width: 1100px) and (min-height: 640px)') : null;
  var current = 0;
  var held = 0;
  var frame = 0;
  var tourReady = false;

  function scrolling() {
    return !!(hero && hero.hasAttribute('data-scroll'));
  }

  function centreTab(tab) {
    if (!strip || strip.scrollWidth <= strip.clientWidth) return;
    var at = tab.getBoundingClientRect().left - strip.getBoundingClientRect().left + strip.scrollLeft;
    strip.scrollTo({
      left: at - (strip.clientWidth - tab.offsetWidth) / 2,
      behavior: reducedMotion ? 'auto' : 'smooth'
    });
  }

  function show(index, focus) {
    if (!tabs.length) return;
    current = index;
    tabs.forEach(function (tab, i) {
      var on = i === index;
      tab.setAttribute('aria-selected', String(on));
      tab.tabIndex = on ? 0 : -1;
      if (panels[i]) {
        panels[i].classList.toggle('is-active', on);
        if (on) panels[i].removeAttribute('inert');
        else panels[i].setAttribute('inert', '');
      }
    });
    if (appTitle) appTitle.textContent = tabs[index].getAttribute('data-window-title') || '';
    if (tourCounter) {
      tourCounter.textContent =
        String(index + 1).padStart(2, '0') + ' / ' + String(tabs.length).padStart(2, '0') +
        ' · ' + (tabs[index].getAttribute('aria-label') || '');
    }
    if (tourPrev) tourPrev.disabled = index === 0;
    if (tourNext) tourNext.disabled = index === tabs.length - 1;
    if (focus) tabs[index].focus();
    centreTab(tabs[index]);
  }

  function pinTop() {
    return pin ? parseFloat(window.getComputedStyle(pin).top) || 0 : 0;
  }

  function room() {
    if (!track || !pin) return 0;
    return Math.max(0, track.offsetHeight - pin.offsetHeight);
  }

  function follow() {
    frame = 0;
    if (!tourReady || !scrolling() || held || !track || !pin) return;
    var travel = room() > 0 ? (pinTop() - track.getBoundingClientRect().top) / room() : 0;
    var last = tabs.length - 1;
    travel = Math.min(1, Math.max(0, travel));
    var reach = travel * last;
    var index = Math.round(reach);
    track.style.setProperty('--at', travel.toFixed(3));
    if (index !== current) show(index);
  }

  function hold() {
    window.clearTimeout(held);
    held = window.setTimeout(function () {
      held = 0;
      follow();
    }, 160);
  }

  function go(index, focus) {
    show(index, focus);
    if (!scrolling() || !track || !pin) return;
    var last = tabs.length - 1;
    if (last <= 0) return;
    var top =
      window.scrollY +
      track.getBoundingClientRect().top -
      pinTop() +
      room() * (index / last);
    hold();
    window.scrollTo({ top: top, behavior: reducedMotion ? 'auto' : 'smooth' });
  }

  function soon() {
    if (held) hold();
    else if (!frame) frame = window.requestAnimationFrame(follow);
  }

  function decideTour() {
    if (!hero || !scrollMedia) return;
    var on = scrollMedia.matches && !reducedMotion;
    if (tourReady) {
      hero.toggleAttribute('data-scroll', on);
    }
    if (strip) strip.setAttribute('aria-orientation', on ? 'vertical' : 'horizontal');
    if (track) {
      track.style.setProperty('--at', on ? String(current / Math.max(1, tabs.length - 1)) : '0');
    }
    soon();
  }

  if (hero && tabs.length && track && pin) {
    tourReady = true;
    tabs.forEach(function (tab, i) {
      tab.addEventListener('click', function () {
        go(i, false);
      });
    });

    if (tourPrev) {
      tourPrev.addEventListener('click', function () {
        go(Math.max(0, current - 1), false);
      });
    }

    if (tourNext) {
      tourNext.addEventListener('click', function () {
        go(Math.min(tabs.length - 1, current + 1), false);
      });
    }

    document.querySelectorAll('[data-try]').forEach(function (button) {
      button.addEventListener('click', function () {
        var action = button.getAttribute('data-try');
        var view = { task: 'editor', sync: 'sync', board: 'board', graph: 'graph', ai: 'ai' }[action];
        var tab = view ? document.getElementById('tab-' + view) : null;
        if (tab) tab.click();

        if (action === 'task') {
          var task = document.querySelector('.ed-check[aria-pressed="false"]');
          if (task) task.click();
          else showToast('All three tasks are done.');
        } else if (action === 'sync') {
          var send = document.querySelector('[data-sync-send]');
          if (send && !send.disabled) send.click();
        } else if (action === 'board') {
          var add = document.querySelector('[data-board-add]');
          if (add) add.click();
        } else if (action === 'graph') {
          var node = document.querySelector('.gn-sync');
          if (node) node.click();
        } else if (action === 'ai') {
          var prompt = document.querySelector('[data-ai-prompt="Find related notes"]');
          if (prompt) prompt.click();
        }
      });
    });

    document.addEventListener('keydown', function (event) {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key < '1' || event.key > '5') return;
      var target = event.target;
      if (target && target.closest && target.closest('input, textarea, select, [contenteditable="true"]')) return;
      var app = document.getElementById('app');
      if (!app) return;
      var rect = app.getBoundingClientRect();
      if (rect.bottom < 0 || rect.top > window.innerHeight) return;
      var index = Number(event.key) - 1;
      if (index >= 0 && index < tabs.length) {
        event.preventDefault();
        go(index, false);
      }
    });

    if (strip) {
      strip.addEventListener('keydown', function (event) {
        var step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
        var next;
        if (step) next = (current + step + tabs.length) % tabs.length;
        else if (event.key === 'Home') next = 0;
        else if (event.key === 'End') next = tabs.length - 1;
        else return;
        event.preventDefault();
        go(next, true);
      });
    }

    var tourStart = document.querySelector('[data-tour-start]');
    if (tourStart) {
      tourStart.addEventListener('click', function (event) {
        event.preventDefault();
        var editorTab = document.getElementById('tab-editor');
        if (editorTab) editorTab.click();
        if (!scrolling()) {
          var app = document.getElementById('app');
          if (app) app.scrollIntoView({ behavior: reducedMotion ? 'auto' : 'smooth', block: 'start' });
        }
      });
    }

    window.addEventListener('scroll', soon, { passive: true });
    window.addEventListener('resize', soon);
    if (scrollMedia) {
      if (scrollMedia.addEventListener) scrollMedia.addEventListener('change', decideTour);
      else if (scrollMedia.addListener) scrollMedia.addListener(decideTour);
    }

    show(0, false);
    hero.toggleAttribute('data-scroll', !!(scrollMedia && scrollMedia.matches && !reducedMotion));
    decideTour();
    follow();
  }

  /* ---------- Editor demo ---------- */

  var editor = document.querySelector('[data-demo="editor"]');
  if (editor) {
    var edStatus = editor.querySelector('[data-ed-status]');
    var edLines = editor.querySelector('[data-ed-lines]');
    var edForm = editor.querySelector('[data-ed-form]');
    var edInput = editor.querySelector('.ed-input');
    var edTitle = editor.querySelector('[data-ed-title]');
    var edCrumbTitle = editor.querySelector('[data-ed-crumb-title]');
    var edTasks = editor.querySelector('.ed-tasks');
    var edMarkdown = editor.querySelector('[data-ed-markdown]');
    var edCode = editor.querySelector('[data-ed-code]');
    var edMenu = editor.querySelector('[data-ed-menu]');
    var edBlocks = editor.querySelector('[data-ed-blocks]');
    var edModes = Array.prototype.slice.call(editor.querySelectorAll('[data-ed-mode]'));
    var edCopy = editor.querySelector('[data-ed-copy]');
    var edTimer = 0;

    var saved = function () {
      if (edStatus) {
        edStatus.classList.add('is-saving');
        window.clearTimeout(edTimer);
        edTimer = window.setTimeout(function () {
          edStatus.classList.remove('is-saving');
        }, 700);
      }
      setWindowStatus('Saved locally', 'local');
    };

    var wireCheck = function (check) {
      check.addEventListener('click', function () {
        var on = check.getAttribute('aria-pressed') === 'true';
        check.setAttribute('aria-pressed', String(!on));
        saved();
      });
    };

    editor.querySelectorAll('.ed-check').forEach(wireCheck);

    var syncTitle = function () {
      var text = edTitle ? (edTitle.textContent || '').replace(/\s+/g, ' ').trim() : '';
      var title = text || 'Untitled';
      if (edCrumbTitle) edCrumbTitle.textContent = title;
      var tab = document.getElementById('tab-editor');
      if (tab) tab.setAttribute('data-window-title', title);
      if (appTitle && document.getElementById('view-editor') && document.getElementById('view-editor').classList.contains('is-active')) {
        appTitle.textContent = title;
      }
      return title;
    };

    if (edTitle) {
      edTitle.addEventListener('input', function () {
        syncTitle();
        saved();
      });
      edTitle.addEventListener('blur', function () {
        if (!edTitle.textContent.trim()) edTitle.textContent = 'Untitled';
        syncTitle();
        saved();
      });
      edTitle.addEventListener('keydown', function (event) {
        if (event.key === 'Enter') {
          event.preventDefault();
          edTitle.blur();
        }
      });
    }

    var closeBlockMenu = function () {
      if (edMenu) edMenu.hidden = true;
      if (edBlocks) edBlocks.setAttribute('aria-expanded', 'false');
    };

    var openBlockMenu = function () {
      if (!edMenu) return;
      edMenu.hidden = false;
      if (edBlocks) edBlocks.setAttribute('aria-expanded', 'true');
    };

    if (edBlocks) {
      edBlocks.addEventListener('click', function () {
        if (!edMenu) return;
        if (edMenu.hidden) openBlockMenu();
        else closeBlockMenu();
      });
    }

    if (edMenu) {
      edMenu.addEventListener('click', function (event) {
        var item = event.target.closest('[data-ed-block]');
        if (!item) return;
        var type = item.getAttribute('data-ed-block');
        var text = edInput ? edInput.value.trim() : '';
        if (edInput) edInput.value = '';

        if (type === 'task') {
          var li = document.createElement('li');
          var check = document.createElement('button');
          check.className = 'ed-check';
          check.type = 'button';
          check.setAttribute('aria-pressed', 'false');
          check.setAttribute('aria-label', 'Mark ' + (text || 'New task') + ' as done');
          var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
          svg.setAttribute('class', 'icon');
          svg.setAttribute('aria-hidden', 'true');
          var use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
          use.setAttribute('href', '#i-check');
          svg.appendChild(use);
          check.appendChild(svg);
          var span = document.createElement('span');
          span.textContent = text || 'New task';
          li.appendChild(check);
          li.appendChild(span);
          edTasks.appendChild(li);
          wireCheck(check);
        } else if (type === 'callout') {
          var callout = document.createElement('div');
          callout.className = 'ed-inline-callout';
          callout.textContent = text || 'A note inside a callout.';
          edLines.appendChild(callout);
        } else if (type === 'heading') {
          var heading = document.createElement('h4');
          heading.className = 'ed-inline-heading';
          heading.textContent = text || 'New heading';
          edLines.appendChild(heading);
        } else if (type === 'divider') {
          var rule = document.createElement('hr');
          rule.className = 'ed-divider';
          edLines.appendChild(rule);
        }

        closeBlockMenu();
        saved();
        if (edInput) edInput.focus();
      });

      document.addEventListener('click', function (event) {
        if (!edMenu || edMenu.hidden) return;
        if (edForm && edForm.contains(event.target)) return;
        if (edMenu.contains(event.target)) return;
        closeBlockMenu();
      });

      document.addEventListener('keydown', function (event) {
        if (event.key !== 'Escape' || edMenu.hidden) return;
        closeBlockMenu();
        if (edBlocks) edBlocks.focus();
      });
    }

    if (edForm && edInput && edLines) {
      edForm.addEventListener('submit', function (event) {
        event.preventDefault();
        var text = edInput.value.trim();
        if (!text) return;
        var line = document.createElement('p');
        line.textContent = text;
        edLines.appendChild(line);
        edInput.value = '';
        edInput.focus();
        saved();
        edLines.scrollTop = edLines.scrollHeight;
      });
    }

    var generateMarkdown = function () {
      var title = syncTitle();
      var tags = Array.prototype.slice.call(editor.querySelectorAll('.ed-tags .tag:not(.tag-add)'))
        .map(function (tag) { return tag.textContent.trim(); })
        .join(' ');
      var calloutEl = editor.querySelector('.ed-callout');
      var callout = calloutEl ? calloutEl.innerText.replace(/\s+/g, ' ').trim() : '';
      var tasks = Array.prototype.slice.call(edTasks.querySelectorAll('li')).map(function (li) {
        var check = li.querySelector('.ed-check');
        var text = li.querySelector('span');
        var done = check && check.getAttribute('aria-pressed') === 'true';
        return '- [' + (done ? 'x' : ' ') + '] ' + (text ? text.textContent.trim() : '');
      });
      var lines = Array.prototype.slice.call(edLines.children).map(function (el) {
        if (el.classList.contains('ed-inline-heading')) return '## ' + el.textContent.trim();
        if (el.classList.contains('ed-inline-callout')) return '> ' + el.textContent.trim();
        if (el.tagName === 'HR') return '---';
        return el.textContent.trim();
      }).filter(Boolean);

      var out = ['# ' + title, ''];
      if (tags) out.push('tags: ' + tags, '');
      if (callout) out.push('> ' + callout, '');
      out.push('## This week', '');
      tasks.forEach(function (task) { out.push(task); });
      if (tasks.length) out.push('');
      lines.forEach(function (line) { out.push(line, ''); });
      return out.join('\n').replace(/\n{3,}/g, '\n\n').trim() + '\n';
    };

    var setMode = function (mode) {
      var markdown = mode === 'markdown';
      if (edCode) edCode.textContent = markdown ? generateMarkdown() : '';
      if (edMarkdown) edMarkdown.hidden = !markdown;
      editor.classList.toggle('is-markdown', markdown);
      edModes.forEach(function (button) {
        button.setAttribute('aria-pressed', String(button.getAttribute('data-ed-mode') === mode));
      });
      closeBlockMenu();
      if (markdown) {
        var pre = edCode ? edCode.closest('pre') : null;
        if (pre) pre.focus();
      } else if (edInput) {
        edInput.focus();
      }
    };

    edModes.forEach(function (button) {
      button.addEventListener('click', function () {
        setMode(button.getAttribute('data-ed-mode'));
      });
    });

    if (edCopy) {
      edCopy.addEventListener('click', function () {
        var text = edCode ? edCode.textContent : '';
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(
            function () { showToast('Markdown copied'); },
            function () { showToast('Could not copy the Markdown'); }
          );
        } else {
          showToast('Select the Markdown and copy it');
        }
      });
    }
  }

  /* ---------- Sync demo ---------- */

  var syncDemo = document.querySelector('[data-demo="sync"]');
  if (syncDemo) {
    var syncButton = syncDemo.querySelector('[data-sync-send]');
    var syncFlow = syncDemo.querySelector('[data-sync-flow]');
    var syncStatus = syncDemo.querySelector('[data-sync-status]');
    var syncForm = syncDemo.querySelector('[data-sync-form]');
    var syncInput = syncDemo.querySelector('.sync-input');
    var syncSend = syncDemo.querySelector('.sync-send');
    var phoneList = syncDemo.querySelector('[data-sync-list="phone"]');
    var desktopList = syncDemo.querySelector('[data-sync-list="desktop"]');
    var desktopCard = desktopList ? desktopList.closest('.device') : null;
    var syncItems = ['Sunscreen', 'Book the ferry', 'Camera', 'Trail map'];
    var syncStep = 0;
    var syncBusy = false;

    var addSyncItem = function (list, text, isNew) {
      if (!list) return null;
      while (list.children.length >= 4) list.removeChild(list.firstElementChild);
      var li = document.createElement('li');
      li.textContent = text;
      if (isNew) li.classList.add('is-new');
      list.appendChild(li);
      return li;
    };

    var setSyncStatus = function (text) {
      if (syncStatus) syncStatus.textContent = text;
    };

    var setSyncDisabled = function (disabled) {
      if (syncButton) syncButton.disabled = disabled;
      if (syncInput) syncInput.disabled = disabled;
      if (syncSend) syncSend.disabled = disabled;
    };

    var runSync = function (item) {
      if (syncBusy || !item) return;
      syncBusy = true;
      setSyncDisabled(true);
      var phoneItem = addSyncItem(phoneList, item, true);
      setSyncStatus('Adding “' + item + '” on the phone…');

      window.setTimeout(function () {
        if (syncFlow) syncFlow.classList.add('is-sending');
        setSyncStatus('Sending one encrypted frame to the desktop…');
      }, reducedMotion ? 120 : 550);

      window.setTimeout(function () {
        var deskItem = addSyncItem(desktopList, item, true);
        setSyncStatus('Desktop applied the change · 18 ms');
        setWindowStatus('Synced · 2 devices', 'synced');
        if (desktopCard) {
          desktopCard.classList.add('is-flash');
          window.setTimeout(function () {
            desktopCard.classList.remove('is-flash');
          }, 600);
        }
        [phoneItem, deskItem].forEach(function (node) {
          if (!node) return;
          window.setTimeout(function () {
            node.classList.remove('is-new');
          }, 900);
        });
      }, reducedMotion ? 240 : 1200);

      window.setTimeout(function () {
        if (syncFlow) syncFlow.classList.remove('is-sending');
        setSyncDisabled(false);
        syncBusy = false;
        if (syncInput) syncInput.focus();
      }, reducedMotion ? 360 : 1900);
    };

    if (syncButton && phoneList && desktopList) {
      syncButton.addEventListener('click', function () {
        var item = syncItems[syncStep % syncItems.length];
        syncStep += 1;
        runSync(item);
      });
    }

    if (syncForm && syncInput && phoneList && desktopList) {
      syncForm.addEventListener('submit', function (event) {
        event.preventDefault();
        var item = syncInput.value.trim();
        if (!item) return;
        syncInput.value = '';
        runSync(item);
      });
    }
  }

  /* ---------- Whiteboard demo ---------- */

  var board = document.querySelector('[data-board]');
  if (board) {
    var boardHint = document.querySelector('[data-board-hint]');
    var addBoardButton = document.querySelector('[data-board-add]');
    var boardSpots = [[12, 58], [38, 58], [12, 76], [38, 76]];

    var clamp = function (value, min, max) {
      return Math.min(Math.max(value, min), max);
    };

    var setBoardHint = function (text) {
      if (boardHint) boardHint.textContent = text;
    };

    var removeSticky = function (sticky) {
      sticky.remove();
      setBoardHint('Sticky removed. Double-click the canvas to add another.');
    };

    var wireRemove = function (sticky) {
      var remove = sticky.querySelector('.sticky-remove');
      if (!remove) return;
      remove.addEventListener('click', function (event) {
        event.stopPropagation();
        removeSticky(sticky);
      });
    };

    var dragSticky = function (sticky) {
      var dragging = false;
      var startX = 0;
      var startY = 0;
      var startLeft = 0;
      var startTop = 0;

      sticky.addEventListener('pointerdown', function (event) {
        if (event.target.closest('.sticky-remove')) return;
        if (event.button !== 0 && event.pointerType === 'mouse') return;
        var rect = sticky.getBoundingClientRect();
        var boardRect = board.getBoundingClientRect();
        startX = event.clientX;
        startY = event.clientY;
        startLeft = rect.left - boardRect.left;
        startTop = rect.top - boardRect.top;
        dragging = true;
        sticky.classList.add('is-dragging');
        sticky.setPointerCapture(event.pointerId);
        event.preventDefault();
      });

      sticky.addEventListener('pointermove', function (event) {
        if (!dragging) return;
        var left = startLeft + (event.clientX - startX);
        var top = startTop + (event.clientY - startY);
        left = clamp(left, 0, Math.max(0, board.clientWidth - sticky.offsetWidth));
        top = clamp(top, 0, Math.max(0, board.clientHeight - sticky.offsetHeight));
        sticky.style.left = left + 'px';
        sticky.style.top = top + 'px';
      });

      var end = function (event) {
        if (!dragging) return;
        dragging = false;
        sticky.classList.remove('is-dragging');
        if (sticky.hasPointerCapture(event.pointerId)) {
          sticky.releasePointerCapture(event.pointerId);
        }
        setBoardHint('Moved. Add another, or drag it further.');
      };

      sticky.addEventListener('pointerup', end);
      sticky.addEventListener('pointercancel', end);

      sticky.addEventListener('keydown', function (event) {
        if (event.key === 'Delete' || event.key === 'Backspace') {
          if (event.target.closest('.sticky-remove')) return;
          event.preventDefault();
          removeSticky(sticky);
          return;
        }
        var keys = { ArrowLeft: [-8, 0], ArrowRight: [8, 0], ArrowUp: [0, -8], ArrowDown: [0, 8] };
        var step = keys[event.key];
        if (!step) return;
        event.preventDefault();
        var rect = sticky.getBoundingClientRect();
        var boardRect = board.getBoundingClientRect();
        var left = clamp(rect.left - boardRect.left + step[0], 0, Math.max(0, board.clientWidth - sticky.offsetWidth));
        var top = clamp(rect.top - boardRect.top + step[1], 0, Math.max(0, board.clientHeight - sticky.offsetHeight));
        sticky.style.left = left + 'px';
        sticky.style.top = top + 'px';
        setBoardHint('Moved with the keyboard. Nice.');
      });
    };

    var addSticky = function (left, top, label, centre) {
      if (board.querySelectorAll('.board-sticky').length >= 5) {
        setBoardHint('Five notes is plenty for the demo.');
        return;
      }

      var sticky = document.createElement('div');
      sticky.className = 'board-sticky';
      sticky.setAttribute('tabindex', '0');
      sticky.setAttribute('role', 'group');
      sticky.setAttribute('aria-roledescription', 'draggable sticky note');
      sticky.setAttribute('aria-label', (label || 'New sticky note') + '. Drag it, use arrow keys, or press Delete to remove.');

      var remove = document.createElement('button');
      remove.className = 'sticky-remove';
      remove.type = 'button';
      remove.setAttribute('aria-label', 'Remove sticky note');
      remove.innerHTML = '<svg class="icon" aria-hidden="true"><use href="#i-trash"/></svg>';

      var strong = document.createElement('strong');
      strong.textContent = label || 'New idea';
      var span = document.createElement('span');
      span.textContent = 'Drag me anywhere';
      var small = document.createElement('small');
      small.textContent = 'drag me';

      sticky.appendChild(remove);
      sticky.appendChild(strong);
      sticky.appendChild(span);
      sticky.appendChild(small);
      board.appendChild(sticky);

      var x = centre ? left - sticky.offsetWidth / 2 : left;
      var y = centre ? top - sticky.offsetHeight / 2 : top;
      sticky.style.left = clamp(x, 0, Math.max(0, board.clientWidth - sticky.offsetWidth)) + 'px';
      sticky.style.top = clamp(y, 0, Math.max(0, board.clientHeight - sticky.offsetHeight)) + 'px';

      wireRemove(sticky);
      dragSticky(sticky);
      sticky.focus();
      setBoardHint('Sticky added. Drag it, or use the arrow keys.');
    };

    board.querySelectorAll('.board-sticky').forEach(function (sticky) {
      wireRemove(sticky);
      dragSticky(sticky);
    });

    if (addBoardButton) {
      addBoardButton.addEventListener('click', function () {
        var count = board.querySelectorAll('.board-sticky').length;
        var spot = boardSpots[count % boardSpots.length];
        addSticky(
          board.clientWidth * (spot[0] / 100),
          board.clientHeight * (spot[1] / 100),
          'New idea',
          false
        );
      });
    }

    board.addEventListener('dblclick', function (event) {
      if (event.target.closest('.board-sticky, .sticky-remove, .board-code, .board-note')) return;
      var rect = board.getBoundingClientRect();
      addSticky(event.clientX - rect.left, event.clientY - rect.top, 'From the canvas', true);
    });

    window.addEventListener('resize', function () {
      board.querySelectorAll('.board-sticky').forEach(function (sticky) {
        var rect = sticky.getBoundingClientRect();
        var boardRect = board.getBoundingClientRect();
        var left = clamp(rect.left - boardRect.left, 0, Math.max(0, board.clientWidth - sticky.offsetWidth));
        var top = clamp(rect.top - boardRect.top, 0, Math.max(0, board.clientHeight - sticky.offsetHeight));
        sticky.style.left = left + 'px';
        sticky.style.top = top + 'px';
      });
    });
  }

  /* ---------- Graph demo ---------- */

  var graph = document.querySelector('[data-graph]');
  if (graph) {
    var graphDetail = document.querySelector('.graph-detail');
    var graphTitle = document.querySelector('[data-graph-title]');
    var graphMeta = document.querySelector('[data-graph-meta]');
    var graphLinks = document.querySelector('[data-graph-links]');
    var graphNodes = Array.prototype.slice.call(graph.querySelectorAll('.graph-node'));
    var graphEdges = Array.prototype.slice.call(graph.querySelectorAll('[data-edge]'));
    var graphSelected = 'roadmap';
    var graphData = {
      design: { title: 'Design system', meta: '2 backlinks · 2 outgoing links', links: ['Meeting notes', 'Sync design'] },
      meeting: { title: 'Meeting notes', meta: '2 backlinks · 1 outgoing link', links: ['Design system'] },
      sync: { title: 'Sync design', meta: '2 backlinks · 1 outgoing link', links: ['Design system'] },
      roadmap: { title: 'Roadmap Q4', meta: '3 backlinks · 3 outgoing links', links: ['Meeting notes', 'Sync design', 'Editor'] },
      editor: { title: 'Editor', meta: '1 backlink · 1 outgoing link', links: ['Roadmap Q4'] },
      board: { title: 'Whiteboard', meta: '1 backlink · 1 outgoing link', links: ['Roadmap Q4'] }
    };

    var highlight = function (name) {
      if (!name) {
        graph.classList.remove('is-focused');
        graphNodes.forEach(function (node) {
          node.classList.remove('is-active', 'is-dim');
        });
        graphEdges.forEach(function (edge) {
          edge.classList.remove('is-lit');
        });
        return;
      }

      var neighbours = {};
      neighbours[name] = true;
      graphEdges.forEach(function (edge) {
        var nodes = edge.getAttribute('data-edge').split(' ');
        if (nodes.indexOf(name) !== -1) {
          nodes.forEach(function (node) {
            neighbours[node] = true;
          });
        }
      });

      graph.classList.add('is-focused');
      graphNodes.forEach(function (node) {
        var id = node.getAttribute('data-node');
        node.classList.toggle('is-active', id === name);
        node.classList.toggle('is-dim', !neighbours[id]);
      });
      graphEdges.forEach(function (edge) {
        var nodes = edge.getAttribute('data-edge').split(' ');
        edge.classList.toggle('is-lit', nodes.indexOf(name) !== -1);
      });
    };

    var select = function (name) {
      var data = graphData[name];
      if (!data) return;
      graphSelected = name;
      if (graphTitle) graphTitle.textContent = data.title;
      if (graphMeta) graphMeta.textContent = data.meta;
      if (graphLinks) {
        graphLinks.textContent = '';
        data.links.forEach(function (label) {
          var li = document.createElement('li');
          li.textContent = label;
          graphLinks.appendChild(li);
        });
      }
      if (graphDetail) graphDetail.setAttribute('data-selected', name);
      highlight(name);
    };

    var graphCanvas = graph.querySelector('[data-graph-canvas]');
    var graphIn = graph.querySelector('[data-graph-in]');
    var graphOut = graph.querySelector('[data-graph-out]');
    var graphReset = graph.querySelector('[data-graph-reset]');
    var graphScale = 1;
    var graphX = 0;
    var graphY = 0;
    var graphPanning = false;
    var graphStartX = 0;
    var graphStartY = 0;
    var graphStartPanX = 0;
    var graphStartPanY = 0;

    var limitGraph = function (value, min, max) {
      return Math.min(Math.max(value, min), max);
    };

    var applyGraphTransform = function () {
      if (!graphCanvas) return;
      graphCanvas.style.transform = 'translate(' + graphX + 'px, ' + graphY + 'px) scale(' + graphScale + ')';
    };

    var zoomGraph = function (factor) {
      graphScale = limitGraph(graphScale * factor, 0.7, 2);
      applyGraphTransform();
    };

    if (graphIn) graphIn.addEventListener('click', function () { zoomGraph(1.18); });
    if (graphOut) graphOut.addEventListener('click', function () { zoomGraph(1 / 1.18); });
    if (graphReset) {
      graphReset.addEventListener('click', function () {
        graphScale = 1;
        graphX = 0;
        graphY = 0;
        applyGraphTransform();
      });
    }

    graph.addEventListener('wheel', function (event) {
      if (!graphCanvas) return;
      event.preventDefault();
      zoomGraph(event.deltaY < 0 ? 1.1 : 0.9);
    }, { passive: false });

    graph.addEventListener('pointerdown', function (event) {
      if (!graphCanvas || event.pointerType !== 'mouse' || event.button !== 0) return;
      if (event.target.closest('.graph-node, .graph-controls')) return;
      graphPanning = true;
      graphStartX = event.clientX;
      graphStartY = event.clientY;
      graphStartPanX = graphX;
      graphStartPanY = graphY;
      graph.classList.add('is-panning');
      graph.setPointerCapture(event.pointerId);
      event.preventDefault();
    });

    graph.addEventListener('pointermove', function (event) {
      if (!graphPanning) return;
      var maxX = graph.clientWidth * 0.45;
      var maxY = graph.clientHeight * 0.45;
      graphX = limitGraph(graphStartPanX + event.clientX - graphStartX, -maxX, maxX);
      graphY = limitGraph(graphStartPanY + event.clientY - graphStartY, -maxY, maxY);
      applyGraphTransform();
    });

    var endGraphPan = function (event) {
      if (!graphPanning) return;
      graphPanning = false;
      graph.classList.remove('is-panning');
      if (graph.hasPointerCapture(event.pointerId)) graph.releasePointerCapture(event.pointerId);
    };

    graph.addEventListener('pointerup', endGraphPan);
    graph.addEventListener('pointercancel', endGraphPan);

    graphNodes.forEach(function (node) {
      var name = node.getAttribute('data-node');
      node.addEventListener('mouseenter', function () {
        highlight(name);
      });
      node.addEventListener('mouseleave', function () {
        highlight(graphSelected);
      });
      node.addEventListener('focus', function () {
        highlight(name);
      });
      node.addEventListener('blur', function () {
        highlight(graphSelected);
      });
      node.addEventListener('click', function () {
        select(name);
      });
    });

    select(graphSelected);
  }

  /* ---------- Local AI demo ---------- */

  var aiDemo = document.querySelector('[data-demo="ai"]');
  if (aiDemo) {
    var aiChat = aiDemo.querySelector('[data-ai-chat]');
    var aiForm = aiDemo.querySelector('[data-ai-form]');
    var aiInput = aiDemo.querySelector('.ai-input');
    var aiSuggestions = Array.prototype.slice.call(aiDemo.querySelectorAll('[data-ai-prompt]'));
    var aiLive = aiDemo.querySelector('[data-ai-live]');
    var aiTimer = 0;
    var aiTyping = null;
    var aiTypingText = '';
    var aiTypingCites = [];

    var scrollChat = function () {
      if (aiChat) aiChat.scrollTop = aiChat.scrollHeight;
    };

    var wireCites = function (scope) {
      scope.querySelectorAll('[data-cite]').forEach(function (button) {
        button.addEventListener('click', function () {
          var node = button.getAttribute('data-cite');
          var graphTab = document.getElementById('tab-graph');
          if (graphTab) graphTab.click();
          var target = document.querySelector('.graph-node[data-node="' + node + '"]');
          if (target) {
            target.click();
            target.focus();
          }
          showToast('Opened ' + (target ? target.textContent.trim() : 'the graph'));
        });
      });
    };

    var addCites = function (message, cites) {
      if (!aiChat || !message || !cites || !cites.length) return;
      var row = document.createElement('div');
      row.className = 'ai-cites';
      var label = document.createElement('span');
      label.textContent = 'Cited';
      row.appendChild(label);
      cites.forEach(function (pair) {
        var button = document.createElement('button');
        button.type = 'button';
        button.setAttribute('data-cite', pair[0]);
        button.textContent = pair[1];
        row.appendChild(button);
      });
      message.insertAdjacentElement('afterend', row);
      wireCites(row);
      scrollChat();
    };

    var finishTyping = function () {
      window.clearTimeout(aiTimer);
      if (aiTyping) {
        aiTyping.textContent = aiTypingText;
        aiTyping.classList.remove('is-typing');
        if (aiLive && aiTypingText) aiLive.textContent = 'Assistant: ' + aiTypingText;
        addCites(aiTyping, aiTypingCites);
        aiTyping = null;
      }
    };

    var aiAnswers = {
      'Summarise this page': {
        text: 'Roadmap Q4 is about shipping encrypted local search, background sync and the split-view polish. Three tasks are still open.',
        cites: [['roadmap', 'Roadmap Q4']]
      },
      'Find related notes': {
        text: 'Two pages mention this work: Meeting notes — design sync, and Sync design. Both are linked from this page.',
        cites: [['meeting', 'Meeting notes'], ['sync', 'Sync design']]
      },
      'Draft a checklist': {
        text: 'Checklist drafted: 1) Encrypted local search, 2) Background sync, 3) Split-view polish, 4) Graph touch targets.',
        cites: [['roadmap', 'Roadmap Q4']]
      }
    };

    var addMessage = function (text, type) {
      if (!aiChat) return null;
      var message = document.createElement('div');
      message.className = 'ai-msg ' + (type === 'user' ? 'ai-user' : 'ai-bot');
      message.textContent = text;
      aiChat.appendChild(message);
      scrollChat();
      return message;
    };

    var answerFor = function (prompt) {
      var exact = aiAnswers[prompt];
      if (exact) return exact;
      var key = Object.keys(aiAnswers).find(function (item) {
        return prompt.toLowerCase().indexOf(item.toLowerCase().split(' ')[0]) !== -1;
      });
      if (key) return aiAnswers[key];
      return {
        text: 'This is a sample answer. In the app, the assistant would answer through the endpoint you configured — with no request leaving the local network unless you choose one that does.',
        cites: [['roadmap', 'Roadmap Q4']]
      };
    };

    var ask = function (prompt) {
      var text = prompt.trim();
      if (!text) return;
      finishTyping();
      addMessage(text, 'user');
      var answer = answerFor(text);
      var reply = addMessage('', 'bot');
      if (!reply) return;
      reply.classList.add('is-typing');
      aiTyping = reply;
      aiTypingText = answer.text;
      aiTypingCites = answer.cites;
      var index = 0;
      if (reducedMotion) {
        reply.textContent = answer.text;
        reply.classList.remove('is-typing');
        aiTyping = null;
        if (aiLive) aiLive.textContent = 'Assistant: ' + answer.text;
        addCites(reply, answer.cites);
        scrollChat();
        return;
      }
      var type = function () {
        index += 2;
        reply.textContent = answer.text.slice(0, index);
        scrollChat();
        if (index < answer.text.length) {
          aiTimer = window.setTimeout(type, 18);
        } else {
          reply.classList.remove('is-typing');
          aiTyping = null;
          if (aiLive) aiLive.textContent = 'Assistant: ' + answer.text;
          addCites(reply, answer.cites);
          scrollChat();
        }
      };
      window.setTimeout(type, 240);
    };

    aiSuggestions.forEach(function (button) {
      button.addEventListener('click', function () {
        ask(button.getAttribute('data-ai-prompt') || button.textContent || '');
      });
    });

    if (aiForm && aiInput) {
      aiForm.addEventListener('submit', function (event) {
        event.preventDefault();
        var value = aiInput.value.trim();
        if (!value) return;
        aiInput.value = '';
        ask(value);
      });
    }

    aiChat.querySelectorAll('.ai-cites').forEach(wireCites);
  }

  /* ---------- Roadmap ---------- */

  var roadmap = document.querySelector('[data-roadmap]');
  if (roadmap) {
    var roadmapTabs = Array.prototype.slice.call(roadmap.querySelectorAll('[role="tab"]'));
    var roadmapPanes = roadmapTabs.map(function (tab) {
      return document.getElementById(tab.getAttribute('aria-controls'));
    });

    var showPhase = function (index, focus) {
      roadmapTabs.forEach(function (tab, i) {
        var on = i === index;
        tab.setAttribute('aria-selected', String(on));
        tab.tabIndex = on ? 0 : -1;
        if (roadmapPanes[i]) {
          roadmapPanes[i].classList.toggle('is-active', on);
          if (on) roadmapPanes[i].removeAttribute('inert');
          else roadmapPanes[i].setAttribute('inert', '');
        }
      });
      if (focus) roadmapTabs[index].focus();
    };

    roadmapTabs.forEach(function (tab, i) {
      tab.addEventListener('click', function () {
        showPhase(i, false);
      });
    });

    var roadmapStrip = roadmap.querySelector('.roadmap-tabs');
    if (roadmapStrip) {
      roadmapStrip.addEventListener('keydown', function (event) {
        var current = roadmapTabs.findIndex(function (tab) {
          return tab.getAttribute('aria-selected') === 'true';
        });
        var step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
        var next;
        if (step) next = (current + step + roadmapTabs.length) % roadmapTabs.length;
        else if (event.key === 'Home') next = 0;
        else if (event.key === 'End') next = roadmapTabs.length - 1;
        else return;
        event.preventDefault();
        showPhase(next, true);
      });
    }

    showPhase(0, false);
  }

  /* ---------- Command palette ---------- */

  var palette = document.getElementById('palette');
  if (palette) {
    var paletteInput = document.getElementById('palette-input');
    var paletteList = document.getElementById('palette-list');
    var paletteEmpty = palette.querySelector('[data-palette-empty]');
    var paletteItems = Array.prototype.slice.call(paletteList.querySelectorAll('li'));
    var paletteActive = 0;
    var paletteOrigin = null;

    paletteItems.forEach(function (item, i) {
      var option = item.querySelector('button');
      if (option) option.id = 'palette-option-' + i;
    });

    var visibleOptions = function () {
      return paletteItems.filter(function (item) {
        return !item.hidden;
      });
    };

    var setPaletteActive = function (index) {
      var items = visibleOptions();
      if (!items.length) {
        paletteActive = -1;
        paletteInput.removeAttribute('aria-activedescendant');
        return;
      }
      paletteActive = (index + items.length) % items.length;
      items.forEach(function (item, i) {
        var option = item.querySelector('button');
        var on = i === paletteActive;
        option.classList.toggle('is-active', on);
        option.setAttribute('aria-selected', String(on));
        if (on) {
          paletteInput.setAttribute('aria-activedescendant', option.id);
          option.scrollIntoView({ block: 'nearest' });
        }
      });
    };

    var filterPalette = function () {
      var query = paletteInput.value.trim().toLowerCase();
      paletteItems.forEach(function (item) {
        var option = item.querySelector('button');
        var haystack = (option.textContent + ' ' + (option.getAttribute('data-keywords') || '')).toLowerCase();
        item.hidden = !!query && haystack.indexOf(query) === -1;
      });
      var any = visibleOptions().length > 0;
      if (paletteEmpty) paletteEmpty.hidden = any;
      setPaletteActive(0);
    };

    var openPalette = function (origin) {
      paletteOrigin = origin && typeof origin.focus === 'function' ? origin : null;
      paletteInput.value = '';
      paletteItems.forEach(function (item) {
        item.hidden = false;
      });
      if (paletteEmpty) paletteEmpty.hidden = true;
      palette.showModal();
      root.classList.add('palette-open');
      paletteInput.setAttribute('aria-expanded', 'true');
      setPaletteActive(0);
      paletteList.scrollTop = 0;
      paletteInput.focus();
    };

    var closePalette = function () {
      if (palette.open) palette.close();
    };

    var scrollToId = function (id) {
      var target = document.getElementById(id);
      if (!target) return;
      target.scrollIntoView({ behavior: reducedMotion ? 'auto' : 'smooth', block: 'start' });
    };

    var runCommand = function (command) {
      var views = {
        editor: 'tab-editor',
        sync: 'tab-sync',
        board: 'tab-board',
        graph: 'tab-graph',
        ai: 'tab-ai'
      };
      var sections = {
        features: 'features',
        security: 'security',
        roadmap: 'roadmap',
        android: 'android',
        faq: 'faq',
        download: 'download',
        docs: 'docs'
      };

      if (command === 'tour') {
        var tourTab = document.getElementById('tab-editor');
        if (tourTab) tourTab.click();
        if (!hero || !hero.hasAttribute('data-scroll')) scrollToId('app');
        return;
      }
      if (views[command]) {
        var tab = document.getElementById(views[command]);
        if (tab) tab.click();
        if (!hero || !hero.hasAttribute('data-scroll')) scrollToId('app');
        return;
      }
      if (command === 'theme') {
        var themeButton = document.querySelector('.theme-toggle');
        if (themeButton) themeButton.click();
        return;
      }
      if (command === 'github') {
        window.location.href = 'https://github.com/Pranesh-Selvaraj/Enclave';
        return;
      }
      if (sections[command]) scrollToId(sections[command]);
    };

    var executePalette = function (option) {
      var command = option.getAttribute('data-command');
      closePalette();
      window.setTimeout(function () {
        runCommand(command);
      }, 0);
    };

    document.querySelectorAll('[data-palette-open]').forEach(function (button) {
      button.addEventListener('click', function () {
        openPalette(button);
      });
    });

    paletteInput.addEventListener('input', filterPalette);

    paletteInput.addEventListener('keydown', function (event) {
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        setPaletteActive(paletteActive + 1);
      } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        setPaletteActive(paletteActive - 1);
      } else if (event.key === 'Enter') {
        event.preventDefault();
        var visible = visibleOptions();
        var item = visible[paletteActive];
        if (item) executePalette(item.querySelector('button'));
      } else if (event.key === 'Escape') {
        event.preventDefault();
        closePalette();
      }
    });

    paletteItems.forEach(function (item) {
      var option = item.querySelector('button');
      if (!option) return;
      option.addEventListener('click', function () {
        executePalette(option);
      });
    });

    palette.addEventListener('click', function (event) {
      if (event.target === palette) closePalette();
    });

    palette.addEventListener('close', function () {
      root.classList.remove('palette-open');
      paletteInput.setAttribute('aria-expanded', 'false');
      if (paletteOrigin) paletteOrigin.focus();
      paletteOrigin = null;
    });

    document.addEventListener('keydown', function (event) {
      if (palette.open) return;
      var target = event.target;
      var typing = target && target.closest && target.closest('input, textarea, select, [contenteditable="true"]');
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        openPalette(target);
        return;
      }
      if (event.key === '/' && !typing && !event.metaKey && !event.ctrlKey && !event.altKey) {
        event.preventDefault();
        openPalette(target);
      }
    });
  }

  /* ---------- Downloads for this computer ---------- */

  function thisOS() {
    var agent = (navigator.userAgent || '').toLowerCase();
    var platform = ((navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '').toLowerCase();
    if (/android/.test(agent)) return 'android';
    if (/iphone|ipad|ipod/.test(agent)) return null;
    if (platform.indexOf('win') !== -1 || agent.indexOf('windows') !== -1) return 'windows';
    if (platform.indexOf('mac') !== -1 || agent.indexOf('mac os') !== -1) return 'mac';
    if (platform.indexOf('linux') !== -1 || agent.indexOf('linux') !== -1) return 'linux';
    return null;
  }

  var os = thisOS();
  if (os) {
    var names = { windows: 'Windows', mac: 'macOS', linux: 'Linux', android: 'Android' };
    document.querySelectorAll('[data-os-label]').forEach(function (label) {
      label.textContent = 'Download for ' + names[os];
    });
    document.querySelectorAll('[data-os-download]').forEach(function (link) {
      link.setAttribute('aria-label', 'Download for ' + names[os]);
    });
    var card = document.querySelector('.download-card[data-os="' + os + '"]');
    if (card) card.classList.add('is-you');
  }

  /* ---------- Android lightbox ---------- */

  var dialog = document.getElementById('lightbox');
  var dialogImage = document.getElementById('lightbox-img');
  var dialogCaption = document.getElementById('lightbox-caption');

  if (dialog && dialogImage && dialogCaption && typeof dialog.showModal === 'function') {
    var shots = document.querySelectorAll('a.phone-shot');

    shots.forEach(function (shot) {
      shot.addEventListener('click', function (event) {
        var source = shot.querySelector('img');
        if (!source) return;
        event.preventDefault();
        dialogImage.setAttribute('src', shot.getAttribute('href'));
        dialogImage.setAttribute('alt', source.getAttribute('alt') || '');
        dialogCaption.textContent = shot.getAttribute('data-caption') || '';
        dialog.showModal();
        root.classList.add('lightbox-open');
      });
    });

    dialog.addEventListener('click', function (event) {
      if (event.target === dialog) dialog.close();
    });

    var close = dialog.querySelector('.lightbox-close');
    if (close) {
      close.addEventListener('click', function () {
        dialog.close();
      });
    }

    dialog.addEventListener('close', function () {
      root.classList.remove('lightbox-open');
    });
  }

  /* ---------- Footer year ---------- */

  var year = document.querySelector('[data-year]');
  if (year) year.textContent = String(new Date().getFullYear());

  /* ---------- Start ---------- */

  setupReveals();
})();
