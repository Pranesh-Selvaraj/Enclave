/* Enclave homepage — progressive enhancement only.
   Everything works without JavaScript except the theme toggle and the
   screenshot lightbox; screenshot links still open the full image. */
(function () {
  var root = document.documentElement;
  var THEME_KEY = 'enclave-site-theme';
  var LIGHT_META = '#f1ede5';
  var DARK_META = '#16130f';
  var toggle = document.querySelector('.theme-toggle');
  var lightQuery = window.matchMedia ? window.matchMedia('(prefers-color-scheme: light)') : null;

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
      meta.setAttribute('content', theme === 'light' ? LIGHT_META : DARK_META);
    }

    if (toggle) {
      var next = theme === 'light' ? 'dark' : 'light';
      var label = 'Switch to ' + next + ' theme';
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

  if (lightQuery && lightQuery.addEventListener) {
    lightQuery.addEventListener('change', function (event) {
      if (!readStoredTheme()) {
        applyTheme(event.matches ? 'light' : 'dark');
      }
    });
  }

  /* Screenshot lightbox — anchors keep working without JavaScript. */
  var dialog = document.getElementById('lightbox');
  var dialogImage = document.getElementById('lightbox-img');
  var dialogCaption = document.getElementById('lightbox-caption');

  if (dialog && dialogImage && dialogCaption && typeof dialog.showModal === 'function') {
    var shots = document.querySelectorAll('a.shot');

    for (var i = 0; i < shots.length; i += 1) {
      shots[i].addEventListener('click', function (event) {
        var source = this.querySelector('img');
        if (!source) {
          return;
        }

        event.preventDefault();
        dialogImage.setAttribute('src', this.getAttribute('href'));
        dialogImage.setAttribute('alt', source.getAttribute('alt') || '');
        dialogCaption.textContent = this.getAttribute('data-caption') || '';
        dialog.showModal();
        root.classList.add('lightbox-open');
      });
    }

    dialog.addEventListener('click', function (event) {
      if (event.target === dialog) {
        dialog.close();
      }
    });

    var closeButton = dialog.querySelector('.lightbox-close');
    if (closeButton) {
      closeButton.addEventListener('click', function () {
        dialog.close();
      });
    }

    dialog.addEventListener('close', function () {
      root.classList.remove('lightbox-open');
    });
  }

  var year = document.querySelector('[data-year]');
  if (year) {
    year.textContent = String(new Date().getFullYear());
  }
})();
