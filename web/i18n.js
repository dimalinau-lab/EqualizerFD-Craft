/**
 * Equalizer FD CRAFT - Internationalization (i18n) Engine
 * Decoupled translation loader with dynamic locale switching and DOM data-binding
 */
(function() {
  const STORAGE_KEY = 'eq_craft_lang';
  const SUPPORTED_LANGS = ['ru', 'en'];

  class I18nManager {
    constructor() {
      let langToUse = null;
      try {
        const urlParam = new URLSearchParams(window.location.search).get('lang');
        if (urlParam && SUPPORTED_LANGS.includes(urlParam)) {
          langToUse = urlParam;
        }
      } catch (_) {}

      if (!langToUse) {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored && SUPPORTED_LANGS.includes(stored)) {
          langToUse = stored;
        } else {
          const sysLang = (navigator.language || (navigator.languages && navigator.languages[0]) || '').toLowerCase();
          langToUse = sysLang.startsWith('ru') ? 'ru' : 'en';
        }
      }

      this.lang = langToUse;
      try { localStorage.setItem(STORAGE_KEY, this.lang); } catch (_) {}
      this.dictionaries = { ru: null, en: null };
      this.isLoaded = false;
      this.readyPromise = null;
    }

    async load(lang) {
      if (!SUPPORTED_LANGS.includes(lang)) lang = 'en';
      if (this.dictionaries[lang]) return this.dictionaries[lang];

      try {
        const res = await fetch(`/locales/${lang}.json?v=${Date.now()}`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();
        this.dictionaries[lang] = data;
        return data;
      } catch (err) {
        console.warn(`[i18n] Failed to load locale "${lang}":`, err);
        return null;
      }
    }

    t(path, params = null, fallback = '') {
      if (!path) return fallback || '';
      const dict = this.dictionaries[this.lang];
      if (!dict) return fallback || path;

      const parts = path.split('.');
      let cur = dict;
      for (const part of parts) {
        if (cur && typeof cur === 'object' && part in cur) {
          cur = cur[part];
        } else {
          return fallback || path;
        }
      }

      if (typeof cur === 'object' && cur !== null) {
        return cur;
      }

      if (typeof cur !== 'string') return fallback || path;

      if (params && typeof params === 'object') {
        return cur.replace(/\{(\w+)\}/g, (match, key) => {
          return key in params ? params[key] : match;
        });
      }

      return cur;
    }

    applyTranslations() {
      // 1. Text elements
      document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        const translated = this.t(key);
        if (translated && typeof translated === 'string' && translated !== key) {
          if (key.endsWith('_html')) {
            el.innerHTML = translated;
          } else {
            el.textContent = translated;
          }
        }
      });

      // 2. Tooltips / Titles
      document.querySelectorAll('[data-i18n-title]').forEach(el => {
        const key = el.getAttribute('data-i18n-title');
        const translated = this.t(key);
        if (translated && typeof translated === 'string' && translated !== key) {
          el.title = translated;
        }
      });

      // 3. Placeholders
      document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        const key = el.getAttribute('data-i18n-placeholder');
        const translated = this.t(key);
        if (translated && typeof translated === 'string' && translated !== key) {
          el.placeholder = translated;
        }
      });

      // 4. Update language checkmarks in menubar
      const chkRu = document.getElementById('chkRu');
      const chkEn = document.getElementById('chkEn');
      if (chkRu) chkRu.textContent = this.lang === 'ru' ? '[+]' : '';
      if (chkEn) chkEn.textContent = this.lang === 'en' ? '[+]' : '';

      // 5. Update HTML lang attribute and document title
      document.documentElement.lang = this.lang;
      const docTitle = this.t('app.title');
      if (docTitle && typeof docTitle === 'string') document.title = docTitle;

      // 6. Broadcast event for canvas / dynamic components redraw
      window.dispatchEvent(new CustomEvent('languageChanged', { detail: { lang: this.lang } }));
    }

    async setLanguage(lang) {
      if (!SUPPORTED_LANGS.includes(lang)) return;
      this.lang = lang;
      try { localStorage.setItem(STORAGE_KEY, lang); } catch (_) {}
      if (!this.dictionaries[lang]) {
        await this.load(lang);
      }
      this.applyTranslations();
      if (window.showToast) {
        window.showToast(this.t('toasts.langChanged', null, lang === 'ru' ? 'Язык изменен на Русский' : 'Language changed to English'));
      }
    }

    async init() {
      if (this.readyPromise) return this.readyPromise;
      this.readyPromise = (async () => {
        await this.load(this.lang);
        this.isLoaded = true;
        this.applyTranslations();

        // Pre-cache other language in background for instant switching
        const otherLang = this.lang === 'ru' ? 'en' : 'ru';
        this.load(otherLang).catch(() => {});

        // Bind menubar click events
        const btnRu = document.getElementById('langRu');
        const btnEn = document.getElementById('langEn');
        if (btnRu) btnRu.addEventListener('click', (e) => {
          e.stopPropagation();
          this.setLanguage('ru');
        });
        if (btnEn) btnEn.addEventListener('click', (e) => {
          e.stopPropagation();
          this.setLanguage('en');
        });
      })();
      return this.readyPromise;
    }
  }

  window.I18N = new I18nManager();
  window.t = (path, params, fallback) => window.I18N.t(path, params, fallback);

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => window.I18N.init());
  } else {
    window.I18N.init();
  }
})();
