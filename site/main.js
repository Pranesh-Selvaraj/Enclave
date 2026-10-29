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
    var edTimer = 0;

    var saved = function () {
      if (!edStatus) return;
      edStatus.classList.add('is-saving');
      window.clearTimeout(edTimer);
      edTimer = window.setTimeout(function () {
        edStatus.classList.remove('is-saving');
      }, 700);
    };

    editor.querySelectorAll('.ed-check').forEach(function (check) {
      check.addEventListener('click', function () {
        var on = check.getAttribute('aria-pressed') === 'true';
        check.setAttribute('aria-pressed', String(!on));
        saved();
      });
    });

    if (edForm && edInput && edLines) {
      edForm.addEventListener('submit', function (event) {
        event.preventDefault();
        var text = edInput.value.trim();
        if (!text) return;
        var line = document.createElement('p');
        line.textContent = text;
        edLines.appendChild(line);
        while (edLines.children.length > 3) edLines.removeChild(edLines.firstElementChild);
        edInput.value = '';
        edInput.focus();
        saved();
        edLines.scrollTop = edLines.scrollHeight;
      });
    }
  }

  /* ---------- Sync demo ---------- */

  var syncDemo = document.querySelector('[data-demo="sync"]');
  if (syncDemo) {
    var syncButton = syncDemo.querySelector('[data-sync-send]');
    var syncFlow = syncDemo.querySelector('[data-sync-flow]');
    var syncStatus = syncDemo.querySelector('[data-sync-status]');
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

    if (syncButton && phoneList && desktopList) {
      syncButton.addEventListener('click', function () {
        if (syncBusy) return;
        syncBusy = true;
        syncButton.disabled = true;
        var item = syncItems[syncStep % syncItems.length];
        syncStep += 1;
        var phoneItem = addSyncItem(phoneList, item, true);
        setSyncStatus('Adding “' + item + '” on the phone…');

        window.setTimeout(function () {
          if (syncFlow) syncFlow.classList.add('is-sending');
          setSyncStatus('Sending one encrypted frame to the desktop…');
        }, reducedMotion ? 120 : 550);

        window.setTimeout(function () {
          var deskItem = addSyncItem(desktopList, item, true);
          setSyncStatus('Desktop applied the change · 18 ms');
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
          syncButton.disabled = false;
          syncBusy = false;
        }, reducedMotion ? 360 : 1900);
      });
    }
  }

  /* ---------- Whiteboard demo ---------- */

  var board = document.querySelector('[data-board]');
  if (board) {
    var boardHint = document.querySelector('[data-board-hint]');
    var addBoardButton = document.querySelector('[data-board-add]');
    var boardCount = 1;

    var clamp = function (value, min, max) {
      return Math.min(Math.max(value, min), max);
    };

    var setBoardHint = function (text) {
      if (boardHint) boardHint.textContent = text;
    };

    var dragSticky = function (sticky) {
      var dragging = false;
      var startX = 0;
      var startY = 0;
      var startLeft = 0;
      var startTop = 0;

      sticky.addEventListener('pointerdown', function (event) {
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
        var maxLeft = board.clientWidth - sticky.offsetWidth;
        var maxTop = board.clientHeight - sticky.offsetHeight;
        left = clamp(left, 0, Math.max(0, maxLeft));
        top = clamp(top, 0, Math.max(0, maxTop));
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

    board.querySelectorAll('.board-sticky').forEach(dragSticky);

    if (addBoardButton) {
      addBoardButton.addEventListener('click', function () {
        if (boardCount >= 5) {
          setBoardHint('Five notes is plenty for the demo.');
          return;
        }
        boardCount += 1;
        var sticky = document.createElement('div');
        sticky.className = 'board-sticky';
        sticky.setAttribute('tabindex', '0');
        sticky.setAttribute('role', 'button');
        sticky.setAttribute('aria-label', 'New sticky note. Drag it or use arrow keys.');

        var strong = document.createElement('strong');
        strong.textContent = 'New idea';
        var span = document.createElement('span');
        span.textContent = 'Drag me anywhere';
        var small = document.createElement('small');
        small.textContent = 'drag me';
        sticky.appendChild(strong);
        sticky.appendChild(span);
        sticky.appendChild(small);

        var spots = [[12, 58], [38, 58], [12, 76], [38, 76]];
        var spot = spots[(boardCount - 2) % spots.length];
        sticky.style.left = spot[0] + '%';
        sticky.style.top = spot[1] + '%';
        board.appendChild(sticky);
        dragSticky(sticky);
        sticky.focus();
        setBoardHint('Sticky added. Drag it, or use the arrow keys.');
      });
    }

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
          node.classList.remove('is-active');
        });
        graphEdges.forEach(function (edge) {
          edge.classList.remove('is-lit');
        });
        return;
      }
      graph.classList.add('is-focused');
      graphNodes.forEach(function (node) {
        node.classList.toggle('is-active', node.getAttribute('data-node') === name);
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

    var finishTyping = function () {
      window.clearTimeout(aiTimer);
      if (aiTyping) {
        aiTyping.textContent = aiTypingText;
        aiTyping.classList.remove('is-typing');
        if (aiLive && aiTypingText) aiLive.textContent = 'Assistant: ' + aiTypingText;
        aiTyping = null;
      }
    };
    var aiAnswers = {
      'Summarise this page': 'Roadmap Q4 is about shipping encrypted local search, background sync and the split-view polish. Three tasks are still open.',
      'Find related notes': 'Two pages mention this work: Meeting notes — design sync, and Roadmap Q2. Both are linked from this page.',
      'Draft a checklist': 'Checklist drafted: 1) Encrypted local search, 2) Background sync, 3) Split-view polish, 4) Graph touch targets.'
    };

    var scrollChat = function () {
      if (aiChat) aiChat.scrollTop = aiChat.scrollHeight;
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
      return 'This is a sample answer. In the app, the assistant would answer through the endpoint you configured — with no request leaving the local network unless you choose one that does.';
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
      aiTypingText = answer;
      var index = 0;
      if (reducedMotion) {
        reply.textContent = answer;
        reply.classList.remove('is-typing');
        aiTyping = null;
        if (aiLive) aiLive.textContent = 'Assistant: ' + answer;
        scrollChat();
        return;
      }
      var type = function () {
        index += 2;
        reply.textContent = answer.slice(0, index);
        scrollChat();
        if (index < answer.length) {
          aiTimer = window.setTimeout(type, 18);
        } else {
          reply.classList.remove('is-typing');
          aiTyping = null;
          if (aiLive) aiLive.textContent = 'Assistant: ' + answer;
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
