// =========================================================================
// Equalizer FD CRAFT — Studio Audio Engine (Desktop App)
// Native Desktop Replica - Pure Typography, Zero Emojis
// =========================================================================

const SAMPLE_RATE = 48000;

// Monotonic Integer ID Generator
let _idCounter = Date.now();
function nextId() {
  return ++_idCounter;
}

// Filter Explanations Dictionary (Task 11)
const FILTER_HINTS = {
  PK: "Параметрический (Колокол): точечная регулировка частоты речи. Позволяет убрать комнатный бубнеж (350-500 Гц) или усилить разборчивость (2-3.5 кГц).",
  HP: "Срез НЧ (High-pass): полностью срезает инфразвук, удары по столу, гул стойки и дыхание ниже выбранной частоты (рекомендуется 70-85 Гц).",
  LP: "Срез ВЧ (Low-pass): срезает высокочастотный шум, писк дросселей и шипение выше выбранной частоты (12-16 кГц).",
  LS: "НЧ полка (Low-shelf): ступенчатый подъем или срез всех низких частот (добавляет голосу плотность и бархатистость).",
  HS: "ВЧ полка (High-shelf): ступенчатый подъем всех высоких частот (добавляет голос в миксе, воздух и четкость согласных).",
  NO: "Вырез помехи (Notch): сверхточный вырез узкой полосы (устраняет паразитный свист кулера или гул электросети 50 Гц)."
};

// Power Icon (Clean centered SVG, zero emojis)
function renderPowerIcon(size = 11) {
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" style="display:block; margin:auto;"><path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/></svg>`;
}

// Multi-Tab Preset State System (Task 10)
const savedTabs = (() => {
  try {
    const raw = localStorage.getItem('resona_tabs_state');
    return raw ? JSON.parse(raw) : null;
  } catch (e) { return null; }
})();

const savedActiveTabId = (() => {
  try {
    return localStorage.getItem('resona_active_tab_id');
  } catch (e) { return null; }
})();

function saveTabsState() {
  try {
    if (state && Array.isArray(state.tabs)) {
      localStorage.setItem('resona_tabs_state', JSON.stringify(state.tabs));
      localStorage.setItem('resona_active_tab_id', state.activeTabId);
    }
  } catch (e) {
    console.warn('Failed to save tabs state to localStorage:', e);
  }
}

// Window unload safeguard
window.addEventListener('beforeunload', () => {
  saveTabsState();
});

function createDefaultBlocks() {
  return [
    { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 },
    {
      id: nextId(),
      kind: 'parametric',
      title: 'Параметрические фильтры (Biquad EQ)',
      enabled: true,
      filters: [
        { id: nextId(), type: 'HP', fc: 80, gain: 0.0, q: 0.7071, enabled: true },
        { id: nextId(), type: 'PK', fc: 1200, gain: 3.5, q: 1.4142, enabled: true },
        { id: nextId(), type: 'HS', fc: 8000, gain: 2.0, q: 0.7071, enabled: true },
      ]
    }
  ];
}

// --- VST Plugins Catalog (APO-PLUGINS Suite) ---
let VST_BASE_DIR = 'D:\\\\testik\\\\vst_plugins\\\\';
fetch('/api/info')
  .then(res => res.json())
  .then(data => {
    if (data && data.vst_dir) {
      VST_BASE_DIR = data.vst_dir;
      const optBundled = document.getElementById('optBundledVst');
      if (optBundled) {
        optBundled.value = data.vst_dir;
        optBundled.textContent = `Встроенная библиотека VST: ${data.vst_dir}`;
      }
    }
  })
  .catch(() => {});

const VST_PLUGINS_CATALOG = {
  reagate: {
    title: 'ReaGate (Cockos Noise Gate)',
    category: 'Динамика и очистка речи',
    dll: 'reagate-standalone.dll',
    desc: 'Профессиональный гейт шума с детектором частоты. Полностью отсекает фоновый шум, стук клавиш клавиатуры, клики мыши и дыхание, когда вы молчите.',
    howItWorks: 'Сигнал проходит только тогда, когда уровень голоса превышает порог (Threshold). Плавная атака и удержание исключают "съедание" окончаний слов.',
    params: [
      { key: 'thresh', label: 'Порог срабатывания (Threshold)', min: -60, max: 0, step: 0.5, unit: 'dB', default: -38 },
      { key: 'attack', label: 'Скорость открытия (Attack)', min: 0.1, max: 50, step: 0.5, unit: 'ms', default: 2 },
      { key: 'hold', label: 'Время удержания (Hold)', min: 0, max: 300, step: 5, unit: 'ms', default: 60 },
      { key: 'release', label: 'Плавность закрытия (Release)', min: 10, max: 600, step: 10, unit: 'ms', default: 150 },
      { key: 'preopen', label: 'Опережение (Pre-open)', min: 0, max: 20, step: 0.5, unit: 'ms', default: 2 }
    ],
    presets: [
      { name: 'Речь / Стрим', values: { thresh: -38, attack: 2, hold: 60, release: 150, preopen: 2 } },
      { name: 'Шумная комната / Клавиатура', values: { thresh: -30, attack: 1, hold: 40, release: 100, preopen: 1 } },
      { name: 'Мягкий гейт для подкаста', values: { thresh: -46, attack: 4, hold: 90, release: 220, preopen: 3 } }
    ]
  },
  reacomp: {
    title: 'ReaComp (Cockos Vocal Compressor)',
    category: 'Динамика и плотность вокала',
    dll: 'reacomp-standalone.dll',
    desc: 'Студийный компрессор с ультранизкой задержкой. Выравнивает громкость речи: тихий шепот делает четким и разборчивым, а громкие крики и смех мягко сдерживает без искажений.',
    howItWorks: 'Сжимает динамический диапазон с коэффициентом Ratio при превышении Threshold. Затем Makeup Gain компенсирует потерю общей громкости.',
    params: [
      { key: 'thresh', label: 'Порог сжатия (Threshold)', min: -40, max: 0, step: 0.5, unit: 'dB', default: -18 },
      { key: 'ratio', label: 'Степень сжатия (Ratio)', min: 1, max: 20, step: 0.1, unit: ':1', default: 3.5 },
      { key: 'attack', label: 'Скорость реакции (Attack)', min: 1, max: 50, step: 0.5, unit: 'ms', default: 5 },
      { key: 'release', label: 'Восстановление (Release)', min: 20, max: 500, step: 5, unit: 'ms', default: 100 },
      { key: 'knee', label: 'Мягкость изгиба (Knee)', min: 0, max: 12, step: 0.5, unit: 'dB', default: 4 },
      { key: 'makeup', label: 'Компенсация громкости (Makeup)', min: 0, max: 24, step: 0.5, unit: 'dB', default: 5 }
    ],
    presets: [
      { name: 'Радиовещание', values: { thresh: -20, ratio: 4.0, attack: 5, release: 100, knee: 4, makeup: 6 } },
      { name: 'Мягкий вокал', values: { thresh: -15, ratio: 2.5, attack: 15, release: 160, knee: 6, makeup: 3 } },
      { name: 'Агрессивный лимитер', values: { thresh: -8, ratio: 12.0, attack: 1, release: 60, knee: 1, makeup: 7 } }
    ]
  },
  reafir: {
    title: 'ReaFIR (Cockos FFT Spectral Subtraction)',
    category: 'Спектральное вычитание шума',
    dll: 'reafir_standalone.dll',
    desc: 'Спектральное БПФ-шумоподавление (FFT Spectral Subtraction). Убирает постоянный монотонный шум: гул кулеров видеокарты, шипение дешевой звуковой карты и шум кондиционера без металлического звука.',
    howItWorks: 'Разбивает сигнал на тысячи частотных полос в спектре. Полосы с низким уровнем отсекаются по спектральному порогу, оставляя голос чистым и естественным.',
    params: [
      { key: 'reduction', label: 'Снижение шума (Noise Reduction)', min: 0, max: 40, step: 1, unit: 'dB', default: 16 },
      { key: 'fftsize', label: 'Размер окна анализа (FFT Size)', min: 512, max: 4096, step: 512, unit: 'pts', default: 1024 },
      { key: 'floor', label: 'Шумовой порог (Floor)', min: -80, max: -20, step: 1, unit: 'dB', default: -55 }
    ],
    presets: [
      { name: 'Шум кулеров ПК', values: { reduction: 14, fftsize: 1024, floor: -50 } },
      { name: 'Глубокая очистка шипения', values: { reduction: 24, fftsize: 2048, floor: -60 } },
      { name: 'Легкая коррекция студии', values: { reduction: 8, fftsize: 512, floor: -45 } }
    ]
  },
  readelay: {
    title: 'ReaDelay (Cockos Multi-tap Delay)',
    category: 'Пространство и задержка',
    dll: 'readelay-standalone.dll',
    desc: 'Высокоточный стерео-дилей с фильтрацией повторов. Создает эхо, объемные пространственные отражения и эффект слэпбэк (slapback) как в профессиональных студиях.',
    howItWorks: 'Задерживает звуковую волну на миллисекунды, фильтрует частоты повторов и подмешивает обратно в мастер-канал.',
    params: [
      { key: 'time', label: 'Время задержки (Time)', min: 10, max: 1000, step: 5, unit: 'ms', default: 180 },
      { key: 'feedback', label: 'Обратная связь (Feedback)', min: 0, max: 90, step: 1, unit: '%', default: 28 },
      { key: 'mix', label: 'Баланс эффекта (Dry/Wet)', min: 0, max: 100, step: 1, unit: '%', default: 22 },
      { key: 'lowpass', label: 'Демпфирование ВЧ (Lowpass)', min: 1000, max: 18000, step: 500, unit: 'Hz', default: 6000 }
    ],
    presets: [
      { name: 'Слэпбэк (Slapback вокал)', values: { time: 85, feedback: 12, mix: 25, lowpass: 7000 } },
      { name: 'Студийное эхо', values: { time: 240, feedback: 35, mix: 30, lowpass: 5000 } },
      { name: 'Космический хвост', values: { time: 480, feedback: 60, mix: 40, lowpass: 4000 } }
    ]
  },
  reaxcomp: {
    title: 'ReaXcomp (Cockos Multiband Compressor)',
    category: 'Многополосная динамика',
    dll: 'reaxcomp-standalone.dll',
    desc: 'Многополосный компрессор для раздельного сжатия низких, средних и высоких частот. Делает звук максимально плотным и "дорогим", как на FM-радиостанциях.',
    howItWorks: 'Кроссовер делит аудио на полосы частот, и каждая полоса сжимается со своим порогом и временем восстановления.',
    params: [
      { key: 'lowthresh', label: 'Порог НЧ (Low Thresh)', min: -40, max: 0, step: 1, unit: 'dB', default: -22 },
      { key: 'midthresh', label: 'Порог СЧ (Mid Thresh)', min: -40, max: 0, step: 1, unit: 'dB', default: -18 },
      { key: 'highthresh', label: 'Порог ВЧ (High Thresh)', min: -40, max: 0, step: 1, unit: 'dB', default: -16 },
      { key: 'overallmakeup', label: 'Мастер-усиление (Makeup)', min: 0, max: 18, step: 0.5, unit: 'dB', default: 4.5 }
    ],
    presets: [
      { name: 'FM-радио вещание', values: { lowthresh: -24, midthresh: -20, highthresh: -18, overallmakeup: 5.0 } },
      { name: 'Плотный стрим', values: { lowthresh: -18, midthresh: -16, highthresh: -14, overallmakeup: 3.5 } },
      { name: 'Сдержанный баланс', values: { lowthresh: -12, midthresh: -10, highthresh: -8, overallmakeup: 2.0 } }
    ]
  },
  basiq: {
    title: 'basiQ (Kuassa Baxandall 3-Band Classic EQ)',
    category: 'Аналоговый винтажный эквалайзер',
    dll: 'basiQ.dll',
    desc: 'Теплый классический 3-полосный винтажный эквалайзер на схеме Баксандала (Baxandall). Придает голосу естественную музыкальность без фазовых искажений.',
    howItWorks: 'Плавные шельфовые кривые с музыкальным наклоном формируют богатый плотный бас и шелковистый благородный верх.',
    params: [
      { key: 'bass', label: 'Низкие частоты (Bass)', min: -12, max: 12, step: 0.5, unit: 'dB', default: 1.5 },
      { key: 'mid', label: 'Средние частоты (Mid)', min: -12, max: 12, step: 0.5, unit: 'dB', default: 1.0 },
      { key: 'treble', label: 'Высокие частоты (Treble)', min: -12, max: 12, step: 0.5, unit: 'dB', default: 2.5 }
    ],
    presets: [
      { name: 'Шелковый вокал', values: { bass: 0.5, mid: 1.0, treble: 3.5 } },
      { name: 'Бархатный диктор', values: { bass: 3.0, mid: 0.5, treble: 2.0 } },
      { name: 'Сквозной (Flat)', values: { bass: 0.0, mid: 0.0, treble: 0.0 } }
    ]
  }
};

// --- Roland BOSS RC-505 Mk2 Effect Suite Catalog ---
const RC505_CATALOG = {
  slicer: {
    title: 'BOSS RC-505 Mk2: Slicer (Beat Repeat)',
    category: 'Ритмический нарезчик бита',
    desc: 'Культовый эффект Roland Slicer: нарезает звук голоса или бита на ритмические отрезки (1/8, 1/16, триоли), создавая пульсирующий стаккато-грув.',
    howItWorks: 'LFO прямоугольной или пилообразной формы периодически приглушает амплитуду сигнала синхронно с темпом.',
    params: [
      { key: 'rate', label: 'Темп нарезки (BPM)', min: 40, max: 240, step: 1, unit: 'BPM', default: 128 },
      { key: 'pattern', label: 'Сетка деления', type: 'select', options: ['1/4 Note', '1/8 Note', '1/16 Note', '1/8 Triplet', '1/32 Beat'], default: '1/16 Note' },
      { key: 'depth', label: 'Глубина нарезки (Depth)', min: 0, max: 100, step: 1, unit: '%', default: 95 },
      { key: 'duty', label: 'Ширина импульса (Gate/Duty)', min: 10, max: 90, step: 1, unit: '%', default: 50 }
    ],
    presets: [
      { name: '16-й чоппер', values: { rate: 128, pattern: '1/16 Note', depth: 95, duty: 50 } },
      { name: 'Триольный глитч', values: { rate: 110, pattern: '1/8 Triplet', depth: 100, duty: 40 } },
      { name: 'Мягкий пульс', values: { rate: 120, pattern: '1/8 Note', depth: 60, duty: 70 } }
    ]
  },
  vinyl: {
    title: 'BOSS RC-505 Mk2: Vinyl Flick & Tape Stop',
    category: 'Винтажное торможение пластинки',
    desc: 'Эмуляция винилового проигрывателя: эффект замедления пластинки рукой (Vinyl Brake), шум иглы, треск и винтажное тепло лампового тракта.',
    howItWorks: 'Плавное замедление частоты дискретизации с экспоненциальной кривой торможения мотора + генератор винилового шороха.',
    params: [
      { key: 'speed', label: 'Скорость торможения (Stop Speed)', min: 100, max: 2000, step: 50, unit: 'ms', default: 450 },
      { key: 'crackle', label: 'Шум и треск винила (Noise)', min: 0, max: 100, step: 1, unit: '%', default: 25 },
      { key: 'flutter', label: 'Детонация скорости (Wow & Flutter)', min: 0, max: 100, step: 1, unit: '%', default: 20 },
      { key: 'tone', label: 'Винтажный срез ВЧ (Tone)', min: 2000, max: 16000, step: 500, unit: 'Hz', default: 5500 }
    ],
    presets: [
      { name: 'Быстрый Tape Stop', values: { speed: 250, crackle: 10, flutter: 15, tone: 8000 } },
      { name: 'Винтажная пластинка 33 RPM', values: { speed: 800, crackle: 40, flutter: 35, tone: 4500 } },
      { name: 'Глубокий виниловый стоп', values: { speed: 1400, crackle: 20, flutter: 50, tone: 3500 } }
    ]
  },
  vocoder: {
    title: 'BOSS RC-505 Mk2: Vocoder & Robot Pitch',
    category: 'Робот-голос и формантный вокодер',
    desc: 'Роботизированный вокодер и квантователь высоты тона. Превращает человеческий голос в электро-робота или жесткий T-Pain автотюн.',
    howItWorks: 'Квантование фазы и спектральных гармоник к фиксированной базовой частоте несущей (Carrier) с гармоническим формантным фильтром.',
    params: [
      { key: 'pitch', label: 'Тон робота (Root Pitch)', min: -24, max: 24, step: 1, unit: 'st', default: 0 },
      { key: 'formant', label: 'Форманта (Gender / Character)', min: -12, max: 12, step: 1, unit: 'st', default: 0 },
      { key: 'robotic', label: 'Жесткость квантования (Robot Depth)', min: 0, max: 100, step: 1, unit: '%', default: 85 },
      { key: 'wet', label: 'Подмес синтеза (Dry/Wet)', min: 0, max: 100, step: 1, unit: '%', default: 80 }
    ],
    presets: [
      { name: 'Daft Punk Робот', values: { pitch: -5, formant: -2, robotic: 95, wet: 90 } },
      { name: 'Электро-вокал', values: { pitch: 0, formant: 2, robotic: 70, wet: 60 } },
      { name: 'Монстр НЧ', values: { pitch: -12, formant: -6, robotic: 80, wet: 75 } }
    ]
  },
  ringmod: {
    title: 'BOSS RC-505 Mk2: Ring Modulator',
    category: 'Металлическая sci-fi модуляция',
    desc: 'Культовый металлический сай-фай звук. Умножает сигнал голоса на синусоидальный генератор, порождая негармонические металлические обертоны.',
    howItWorks: 'Математическое перемножение частот f1 +/- f2, убирающее базовую гармонику и создающее колокольные и кибернетические резонансы.',
    params: [
      { key: 'carrier', label: 'Частота модуляции (Carrier Freq)', min: 50, max: 3000, step: 10, unit: 'Hz', default: 440 },
      { key: 'moddepth', label: 'Интенсивность модуляции (Depth)', min: 0, max: 100, step: 1, unit: '%', default: 75 },
      { key: 'mix', label: 'Баланс эффекта (Dry/Wet)', min: 0, max: 100, step: 1, unit: '%', default: 50 }
    ],
    presets: [
      { name: 'Киборг / Далек', values: { carrier: 180, moddepth: 90, mix: 70 } },
      { name: 'Колокольный звон', values: { carrier: 620, moddepth: 60, mix: 40 } },
      { name: 'Инопланетный свист', values: { carrier: 1450, moddepth: 85, mix: 55 } }
    ]
  },
  isolator: {
    title: 'BOSS RC-505 Mk2: Isolator & DJ Filter',
    category: 'DJ фильтрация и kill-переключатели',
    desc: 'Фирменный DJ-изолятор с крутыми срезами 24 dB/oct. Позволяет мгновенно вырезать весь бас, средние или верх для создания мощных дропов.',
    howItWorks: 'Трехполосный кроссовер Linkwitz-Riley с полным глушением (Kill) полос до -Infinity dB и резонансным пиком на срезе.',
    params: [
      { key: 'lowkill', label: 'Низкие частоты (Low Kill)', min: -30, max: 6, step: 0.5, unit: 'dB', default: 0 },
      { key: 'midkill', label: 'Средние частоты (Mid Kill)', min: -30, max: 6, step: 0.5, unit: 'dB', default: 0 },
      { key: 'highkill', label: 'Высокие частоты (High Kill)', min: -30, max: 6, step: 0.5, unit: 'dB', default: 0 },
      { key: 'cutoff', label: 'DJ Sweep Cutoff', min: 100, max: 15000, step: 100, unit: 'Hz', default: 15000 }
    ],
    presets: [
      { name: 'Bass Kill (Дроп)', values: { lowkill: -30, midkill: 0, highkill: 0, cutoff: 15000 } },
      { name: 'Club Filter Sweep', values: { lowkill: 0, midkill: -6, highkill: -18, cutoff: 800 } },
      { name: 'Телефонная трубка', values: { lowkill: -30, midkill: 4, highkill: -30, cutoff: 3500 } }
    ]
  },
  lofi: {
    title: 'BOSS RC-505 Mk2: Lo-Fi Radio & Bitcrush',
    category: 'Lo-Fi рация, мегафон и биткрашер',
    desc: 'Звучание рации спецназа, полицейского мегафона или старого АМ-приемника с характерным хрипом и перегрузом.',
    howItWorks: 'Полосовой фильтр 500-3500 Гц + асимметричный софт-клиппер с легким белым шумом эфира.',
    params: [
      { key: 'drive', label: 'Перегруз рации (Drive)', min: 0, max: 100, step: 1, unit: '%', default: 45 },
      { key: 'lowcut', label: 'Срез НЧ (Low Cut)', min: 200, max: 1200, step: 50, unit: 'Hz', default: 600 },
      { key: 'highcut', label: 'Срез ВЧ (High Cut)', min: 2000, max: 6000, step: 100, unit: 'Hz', default: 3500 },
      { key: 'noise', label: 'Шум эфира (Static Noise)', min: 0, max: 100, step: 1, unit: '%', default: 20 }
    ],
    presets: [
      { name: 'Рация Walkie-Talkie', values: { drive: 55, lowcut: 700, highcut: 3200, noise: 30 } },
      { name: 'Уличный мегафон', values: { drive: 75, lowcut: 400, highcut: 4500, noise: 10 } },
      { name: 'АМ-радио 1940-х', values: { drive: 30, lowcut: 500, highcut: 2600, noise: 40 } }
    ]
  },
  chorus: {
    title: 'BOSS RC-505 Mk2: Space Chorus & Flanger',
    category: 'Пространственный стерео-хорус',
    desc: 'Легендарный хорус Roland Dimension / BOSS CE-1: расширяет моносигнал микрофона в широкую кинематографичную стерео-панораму.',
    howItWorks: 'Модуляция линии задержки синусоидальным LFO с противофазным смещением левого и правого каналов.',
    params: [
      { key: 'rate', label: 'Скорость модуляции (Rate)', min: 0.1, max: 8.0, step: 0.1, unit: 'Hz', default: 1.2 },
      { key: 'depth', label: 'Глубина расширения (Depth)', min: 0, max: 100, step: 1, unit: '%', default: 65 },
      { key: 'feedback', label: 'Резонанс (Flanger Feed)', min: 0, max: 85, step: 1, unit: '%', default: 15 },
      { key: 'mix', label: 'Баланс эффекта (Dry/Wet)', min: 0, max: 100, step: 1, unit: '%', default: 45 }
    ],
    presets: [
      { name: 'Стерео-вокал', values: { rate: 0.8, depth: 60, feedback: 0, mix: 40 } },
      { name: 'Реактивный флэнжер', values: { rate: 0.4, depth: 85, feedback: 70, mix: 55 } },
      { name: 'Вибрато', values: { rate: 4.5, depth: 70, feedback: 0, mix: 100 } }
    ]
  },
  distortion: {
    title: 'BOSS RC-505 Mk2: Distortion & Overdrive',
    category: 'Ламповый перегруз и сатурация',
    desc: 'Ламповый овердрайв и агрессивный дисторшн для придания вокалу или битбоксу мощи, рычания и рокового напора.',
    howItWorks: 'Динамическая кривая насыщения tanh(x) с генерацией четных и нечетных гармоник и пост-фильтром тембра.',
    params: [
      { key: 'drive', label: 'Уровень перегруза (Gain)', min: 0, max: 100, step: 1, unit: '%', default: 35 },
      { key: 'tone', label: 'Тембр (Tone)', min: 1000, max: 12000, step: 200, unit: 'Hz', default: 4500 },
      { key: 'bottom', label: 'Плотность баса (Bass Boost)', min: -10, max: 12, step: 0.5, unit: 'dB', default: 2.0 },
      { key: 'level', label: 'Выходной уровень (Output)', min: -15, max: 6, step: 0.5, unit: 'dB', default: -2.0 }
    ],
    presets: [
      { name: 'Теплый ламповый сатуратор', values: { drive: 20, tone: 6000, bottom: 2.0, level: 0.0 } },
      { name: 'Битбокс бочка/снэр', values: { drive: 45, tone: 4000, bottom: 5.0, level: -2.0 } },
      { name: 'Хардкор дисторшн', values: { drive: 85, tone: 3200, bottom: 0.0, level: -5.0 } }
    ]
  },
  scatter: {
    title: 'BOSS RC-505 Mk2: Granular Beat Scatter',
    category: 'Гранулярный рассеиватель и глитч',
    desc: 'Скаттер-эффект из серии Roland AIRA: микро-зацикливание звуковых гранул, реверсирование и глитч-сбивки на лету.',
    howItWorks: 'Буфер захватывает кусочки звука (гранулы 20-200 мс) и перетасовывает их в псевдослучайном ритмическом порядке.',
    params: [
      { key: 'grain', label: 'Размер гранул (Grain Size)', min: 20, max: 400, step: 10, unit: 'ms', default: 120 },
      { key: 'scattertype', label: 'Режим скаттера', type: 'select', options: ['Случайный скаттер', 'Прямой репит', 'Реверс гранул', 'Питч-шифт вверх'], default: 'Случайный скаттер' },
      { key: 'density', label: 'Плотность сбивки (Density)', min: 10, max: 100, step: 5, unit: '%', default: 70 },
      { key: 'mix', label: 'Баланс эффекта (Dry/Wet)', min: 0, max: 100, step: 1, unit: '%', default: 50 }
    ],
    presets: [
      { name: 'Гранулярный репит', values: { grain: 80, scattertype: 'Прямой репит', density: 80, mix: 60 } },
      { name: 'Реверсивный глитч', values: { grain: 160, scattertype: 'Реверс гранул', density: 65, mix: 50 } },
      { name: 'Космический глитч', values: { grain: 240, scattertype: 'Питч-шифт вверх', density: 75, mix: 45 } }
    ]
  },
  reverb: {
    title: 'BOSS RC-505 Mk2: Space Reverb & Shimmer',
    category: 'Студийный ревербератор BOSS',
    desc: 'Огромная соборная реверберация Roland Space Echo с добавлением мерцающих высотных гармоник (Shimmer).',
    howItWorks: 'Диффузная сеть линий задержки (FDN) с регенеративным питанием и демпфированием высоких частот.',
    params: [
      { key: 'time', label: 'Время затухания (Decay)', min: 0.5, max: 15.0, step: 0.5, unit: 's', default: 3.5 },
      { key: 'predelay', label: 'Предзадержка (Pre-delay)', min: 0, max: 150, step: 5, unit: 'ms', default: 25 },
      { key: 'shimmer', label: 'Мерцание Шиммер (Shimmer)', min: 0, max: 100, step: 1, unit: '%', default: 30 },
      { key: 'mix', label: 'Баланс эффекта (Dry/Wet)', min: 0, max: 100, step: 1, unit: '%', default: 25 }
    ],
    presets: [
      { name: 'Вокальная комната', values: { time: 1.8, predelay: 15, shimmer: 0, mix: 20 } },
      { name: 'Собор (Cathedral)', values: { time: 5.5, predelay: 35, shimmer: 20, mix: 35 } },
      { name: 'Бесконечный Шиммер', values: { time: 12.0, predelay: 50, shimmer: 70, mix: 45 } }
    ]
  }
};

// --- Complete 98 VST Plugins Catalog from P4RKER1/APO-PLUGINS ---
const ALL_VST_98_CATALOG = [
  // --- Dynamics & Compressors ---
  { id: 'reagate', title: 'ReaGate', dll: 'reagate-standalone.dll', cat: 'dynamics', catName: 'Шумоподавление и гейты', desc: 'Профессиональный гейт шума с детекцией частот. Отсекает клики мыши, стук клавиатуры и комнатный шум в паузах речи.', how: 'Пропускает аудиосигнал только при превышении заданного порога громкости (Threshold).' },
  { id: 'reacomp', title: 'ReaComp', dll: 'reacomp-standalone.dll', cat: 'dynamics', catName: 'Компрессоры и плотность', desc: 'Студийный компрессор с ультранизкой задержкой. Сглаживает перепады громкости между тихим шепотом и криком.', how: 'Сжимает динамический диапазон с коэффициентом Ratio и компенсирует общую громкость Makeup Gain.' },
  { id: 'reaxcomp', title: 'ReaXcomp', dll: 'reaxcomp-standalone.dll', cat: 'dynamics', catName: 'Многополосная динамика', desc: '4-полосный кроссовер-компрессор. Делает голос плотным и насыщенным как на радиостанциях.', how: 'Разделяет аудио на 4 частотных диапазона и сжимает каждый независимо.' },
  { id: 'camelcrusher', title: 'CamelCrusher', dll: 'CamelCrusher.dll', cat: 'dynamics', catName: 'Компрессоры и перегруз', desc: 'Культовый британский "fat" компрессор и двухрежимный сатуратор (Tube/Mech). Добавляет голосу мощь и плотность.', how: 'Сочетает нелинейное насыщение кривой лампы с агрессивным Phat-компрессором.' },
  { id: 'mjucjr', title: 'Klanghelm MJUC jr.', dll: 'MJUCjr.dll', cat: 'dynamics', catName: 'Ламповый компрессор', desc: 'Теплый ламповый компрессор Variable-Mu. Идеален для дикторского голоса и вокала.', how: 'Плавное мягкое колено (soft-knee) с винтажным насыщением трансформатора.' },
  { id: 'dc1a3', title: 'Klanghelm DC1A3', dll: 'DC1A3.dll', cat: 'dynamics', catName: 'Характерный компрессор', desc: 'Простой двухручечный компрессор с отличным аналоговым окрасом.', how: 'Интеллектуальная подстройка атаки и релиза под динамику входного сигнала.' },
  { id: 'ggate', title: 'GVST GGate', dll: 'GGate.dll', cat: 'dynamics', catName: 'Шумоподавитель', desc: 'Легковесный гейт для чистки фона микрофона без задержки.', how: 'Классический линейный порог с настраиваемыми атакой и затуханием.' },
  { id: 'clipshifter', title: 'LVC-Audio ClipShifter', dll: 'ClipShifter_x64_1.dll', cat: 'dynamics', catName: 'Динамический клиппер', desc: 'Волновой клиппер и лимитер для отсечения резких взрывных согласных и пиков.', how: 'Срезает пиковые всплески амплитуды с мягким насыщением гармоник.' },
  { id: 'clipmaxx', title: 'Clipmaxx', dll: 'Clipmaxx64.dll', cat: 'dynamics', catName: 'Мягкий клиппер', desc: 'Мастеринговый клиппер для мягкого ограничения пиков без цифрового треска.', how: 'Мягкая полиномиальная кривая ограничения формы волны.' },
  { id: 'deftcomp', title: 'Voxengo Deft Compressor', dll: 'Deft Compressor.dll', cat: 'dynamics', catName: 'Шинный компрессор', desc: 'Прозрачный шинный компрессор с адаптивным выпуском.', how: 'Регулирует компрессию на основе RMS и пикового детектора.' },
  { id: 'free87comp', title: 'eaReckon FREE87 FR-COMP', dll: 'FREE87 FR-COMP(x64).dll', cat: 'dynamics', catName: 'Аналоговый компрессор', desc: 'Аналоговая консольная компрессия серии 80-х годов.', how: 'VCA-эмуляция с минимальным фазовым сдвигом.' },
  { id: 'transgainer', title: 'Voxengo TransGainer', dll: 'TransGainer.dll', cat: 'dynamics', catName: 'Транзиент-шейпер', desc: 'Усилитель или ослабитель транзиентов (щелчков и затухания).', how: 'Детектирует фронты волны и усиливает атаку или тело звука.' },
  { id: 'outlaw', title: 'W.A. Production Outlaw', dll: 'Outlaw_1_1.dll', cat: 'dynamics', catName: 'Авто-гейн / Вокал райдер', desc: 'Автоматический выравниватель уровня речи (Vocal Rider).', how: 'Непрерывно подстраивает гейн, удерживая голос на одном уровне без изменения динамики.' },
  { id: 'vitaminc', title: 'Analog Obsession Vitamin C', dll: 'Vitamin C x64.dll', cat: 'dynamics', catName: 'Опто-лимитер', desc: 'Оптический левелер и компрессор в стиле классического LA-2A.', how: 'Медленная фоторезистивная кривая сжатия для максимальной естественности.' },
  { id: 'shumovick', title: 'Voxengo CRTIV Shumovick', dll: 'CRTIV Shumovick.dll', cat: 'dynamics', catName: 'Динамический шумоподавитель', desc: 'Умный очиститель пауз и речевого тракта от шорохов.', how: 'Глушит тихие промежутки с сохранением естественного шлейфа.' },
  { id: 'ebuslim', title: 'Voxengo EBusLim', dll: 'EBusLim.dll', cat: 'dynamics', catName: 'Шинный лимитер', desc: 'Кирпичный лимитер для защиты динамиков от перегрузок и клиппинга.', how: 'Упреждающий lookahead ограничитель пиков.' },
  { id: 'elephant', title: 'Voxengo Elephant', dll: 'Elephant.dll', cat: 'dynamics', catName: 'Мастеринг-лимитер', desc: 'Топовый мастеринговый лимитер с кристальной прозрачностью.', how: 'Многократная передискретизация без потерь тембра.' },

  // --- Equalizers & Tonal Shaping ---
  { id: 'tdrnova', title: 'TDR Nova', dll: 'TDR_Nova.dll', cat: 'eq', catName: 'Параллельный динамический EQ', desc: 'Параллельный динамический эквалайзер. Убирает бубнеж и резонансы только тогда, когда они появляются.', how: 'Каждая полоса может работать как обычный параметрик или как динамический компрессор частоты.' },
  { id: 'basiq', title: 'Kuassa basiQ', dll: 'basiQ.dll', cat: 'eq', catName: 'Винтажный Baxandall EQ', desc: 'Аналоговый 3-полосный винтажный эквалайзер на схеме Баксандала.', how: 'Музыкальные плавные кривые для теплого баса и прозрачных высоких.' },
  { id: 'reaeq', title: 'ReaEQ', dll: 'reaeq-standalone.dll', cat: 'eq', catName: 'Хирургический параметрик', desc: 'Студийный многополосный параметрический эквалайзер с поддержкой любых типов фильтров.', how: 'Прямой расчет биквадратных фильтров IIR 64-бит.' },
  { id: 'pteqx', title: 'Ignite Amps PTEq-X', dll: 'PTEq-X.dll', cat: 'eq', catName: 'Ламповый Pultec EQ', desc: 'Эмуляция легендарных ламповых эквалайзеров Pultec EQP-1A, MEQ-5 и HLF-3C.', how: 'Моделирование аналоговых ламп 12AX7/ECC83 и пассивных индукционных катушек.' },
  { id: 'tbequalizer', title: 'ToneBoosters TB_Equalizer v3', dll: 'TB_Equalizer_v3_1.dll', cat: 'eq', catName: 'Динамический эквалайзер', desc: 'Гибкий эквалайзер с аналоговым моделированием насыщения полос.', how: 'Сверхточная интерполяция кривых с минимальной фазовой задержкой.' },
  { id: 'primeeq', title: 'Voxengo PrimeEQ', dll: 'PrimeEQ.dll', cat: 'eq', catName: 'Параметрический EQ', desc: 'Простой и надежный эквалайзер для быстрой тональной подстройки.', how: 'Традиционные фильтры с низким потреблением ЦП.' },
  { id: 'harmonieq', title: 'Voxengo HarmoniEQ', dll: 'HarmoniEQ.dll', cat: 'eq', catName: 'Гармонический эквалайзер', desc: 'Эквалайзер, добавляющий гармоники в усиливаемые частоты.', how: 'Генерирует обертоны для придания живости и презенса речи.' },
  { id: 'anspec', title: 'Voxengo AnSpec', dll: 'AnSpec.dll', cat: 'eq', catName: 'Аналоговый спектроанализатор', desc: 'Визуализатор частотного спектра в реальном времени.', how: 'Отображает полосы частот в логарифмической шкале.' },
  { id: 'span', title: 'Voxengo SPAN', dll: 'SPAN.dll', cat: 'eq', catName: 'FFT Спектроанализатор', desc: 'Мировой стандарт FFT-анализа спектра аудиосигнала.', how: 'Быстрое преобразование Фурье высокого разрешения.' },
  { id: 'spanplus', title: 'Voxengo SPAN Plus', dll: 'SPAN Plus.dll', cat: 'eq', catName: 'Спектроанализатор Pro', desc: 'Расширенная версия SPAN с экспортом спектра и статическими пиками.', how: 'Многоканальный спектральный оверлей.' },
  { id: 'bcfreq', title: 'Blue Cat FreqAnalyst 2', dll: 'BC FreqAnalyst 2 VST(Stereo).dll', cat: 'eq', catName: 'Спектральный анализатор', desc: 'Плавный анализатор частот со спектрограммой.', how: 'Интерполяция кривой спектра с высокой частотой кадров.' },
  { id: 'bcfreqm', title: 'Blue Cat FreqAnalyst Mono', dll: 'BC FreqAnalyst 2 VST(Mono).dll', cat: 'eq', catName: 'Моно-анализатор', desc: 'Моно-версия анализатора для экономии ресурсов.', how: 'Анализ единственного аудиоканала.' },
  { id: 'dynfreqlim', title: 'Dynamic Frequency Limiter', dll: 'Dynamic Frequency Limiter.dll', cat: 'eq', catName: 'Де-эссер / Лимитер ВЧ', desc: 'Убирает резкие шипящие согласные (С, Ц, Щ) без приглушения голоса.', how: 'Динамически сжимает только диапазон 5-9 кГц при свисте.' },
  { id: 'filterjam', title: 'AudioThing Filterjam', dll: 'Filterjam_x64.dll', cat: 'eq', catName: 'Резонансный фильтр', desc: 'Многополосный резонансный фильтр для создания необычных sci-fi тембров.', how: 'Кольцевое сложение нескольких резонансных пиков.' },

  // --- Reverb, Space & Delays ---
  { id: 'vintageverb', title: 'Valhalla VintageVerb', dll: 'ValhallaVintageVerb_x64.dll', cat: 'space', catName: 'Студийный ревербератор', desc: 'Легендарный ревербератор 70-80-х годов: от компактной комнаты до бесконечного космического пространства.', how: 'Диффузная регенеративная сеть линий задержки с цветовыми эпохами (1970s, 1980s, NOW).' },
  { id: 'freqecho', title: 'Valhalla FreqEcho', dll: 'ValhallaFreqEcho_x64.dll', cat: 'space', catName: 'Частотный дилей / Эхо', desc: 'Психоделический дилей со встроенным частотным шифтером (Bode Frequency Shifter).', how: 'Сдвигает частоту повторов вверх или вниз на каждый цикл эха.' },
  { id: 'talreverb4', title: 'TAL-Reverb-4', dll: 'TAL-Reverb-4-64.dll', cat: 'space', catName: 'Винтажный плейт-реверб', desc: 'Густой и шелковистый винтажный Plate ревербератор 80-х для вокала.', how: 'Диффузная структура с модуляцией хвоста для устранения металлического призвука.' },
  { id: 'taldub2', title: 'TAL-Dub-2', dll: 'TAL-Dub-2.dll', cat: 'space', catName: 'Ленточный дилей (Tape Echo)', desc: 'Винтажный аналоговый ленточный дилей с ламповым перегрузом и сатурацией.', how: 'Эмуляция магнитной ленты с искажением при высоких значениях feedback.' },
  { id: 'readelay', title: 'ReaDelay', dll: 'readelay-standalone.dll', cat: 'space', catName: 'Многоотводный дилей', desc: 'Студийный стерео-дилей с независимой фильтрацией отводов (taps).', how: 'Миллисекундная и нотная синхронизация с LPF/HPF фильтрацией.' },
  { id: 'dflroom', title: 'Dragonfly Room Reverb', dll: 'DragonflyRoomReverb-vst.dll', cat: 'space', catName: 'Комнатная реверберация', desc: 'Натуральная акустика студийных помещений и залов.', how: 'Свободный алгоритм Freeverb3 с расширенной ранней диффузией.' },
  { id: 'dflhall', title: 'Dragonfly Hall Reverb', dll: 'DragonflyHallReverb-vst.dll', cat: 'space', catName: 'Концертный зал', desc: 'Пространство больших концертных залов и соборов.', how: 'Глубокая рециркуляция с демпфированием высоких частот.' },
  { id: 'dflplate', title: 'Dragonfly Plate Reverb', dll: 'DragonflyPlateReverb-vst.dll', cat: 'space', catName: 'Листовой ревербератор', desc: 'Яркий металлический листовой ревербератор для вокала.', how: 'Высокая плотность начальных отражений.' },
  { id: 'dflearly', title: 'Dragonfly Early Reflections', dll: 'DragonflyEarlyReflections-vst.dll', cat: 'space', catName: 'Ранние отражения', desc: 'Формирование ощущения объема комнаты без длинного хвоста.', how: 'Расчет первых отражений от стен и потолка.' },
  { id: 'oldskoolverb', title: 'Voxengo OldSkoolVerb Plus', dll: 'OldSkoolVerb Plus.dll', cat: 'space', catName: 'Классический реверб', desc: 'Традиционный алгоритмический ревербератор без фазовых конфликтов.', how: 'Классическая схема Шрёдера-Муррера.' },
  { id: 'crtivreverb', title: 'Voxengo CRTIV Reverb 2', dll: 'CRTIV Reverb 2.dll', cat: 'space', catName: 'Творческий реверб', desc: 'Пространственный ревербератор с широкой стерео-базой.', how: 'Стереофоническая пространственная диффузия.' },
  { id: 'muverb', title: 'MuTools MuVerb', dll: 'MuVerb.dll', cat: 'space', catName: 'Компактный реверб', desc: 'Легкий и сочный ревербератор с минимальной нагрузкой на процессор.', how: 'Оптимизированная матрица задержек.' },
  { id: 'deverberate', title: 'Acon Digital DeVerberate 3', dll: 'DeVerberate3.dll', cat: 'space', catName: 'Подавитель реверберации', desc: 'Убирает эхо комнаты и гул помещения из записи микрофона.', how: 'Спектральное разделение прямого звука и диффузного хвоста.' },

  // --- Chorus, Flanger & Modulation ---
  { id: 'talchorus', title: 'TAL-Chorus-LX', dll: 'TAL-Chorus-LX-64.dll', cat: 'mod', catName: 'Аналоговый хорус Juno-60', desc: 'Точная эмуляция культового стерео-хоруса синтезатора Roland Juno-60.', how: 'Две линии задержки BBD (Bucket Brigade Device) с противофазными LFO.' },
  { id: 'talbitcrush', title: 'TAL-Bitcrusher', dll: 'TAL-Bitcrusher.dll', cat: 'mod', catName: 'Винтажный биткрашер', desc: 'Эмуляция старых 8-битных и 12-битных сэмплеров 80-х и винтажного хруста.', how: 'Снижение разрядности (Bit Depth 4-16 bit) и субдискретизация (Downsampling).' },
  { id: 'azurite', title: 'Azurite Multi-Chorus', dll: 'Azurite x64.dll', cat: 'mod', catName: 'Многоголосный хорус', desc: 'Пышный хорус с несколькими голосами для широкого вокала.', how: 'До 8 линий модулированной задержки со случайной фазой.' },
  { id: 'bcchorus', title: 'Blue Cat Chorus 4 Stereo', dll: 'BC Chorus 4 VST(Stereo).dll', cat: 'mod', catName: 'Стерео-хорус', desc: 'Универсальный хорус от легкого расширения до глубокого вибрато.', how: 'Плавная модуляция задержки с контролем стерео-ширины.' },
  { id: 'bcchorusm', title: 'Blue Cat Chorus 4 Mono', dll: 'BC Chorus 4 VST(Mono).dll', cat: 'mod', catName: 'Моно-хорус', desc: 'Классический моно-хорус.', how: 'Одна линия модулированной задержки.' },
  { id: 'multiply', title: 'Acon Digital Multiply', dll: 'Multiply.dll', cat: 'mod', catName: 'Вокальный хорус', desc: 'Микро-модулированный хорус для уплотнения вокала.', how: 'Фазовое рандомизирование с эквализацией влажного сигнала.' },
  { id: 'dimexp', title: 'Xfer Dimension Expander', dll: 'DimensionExpander_x64.dll', cat: 'mod', catName: 'Пространственный экспандер', desc: 'Культовый эффект расширения звука от Steve Duda (Xfer Records).', how: '4 перекрестно отраженные задержки с инверсией фаз.' },
  { id: 'temper', title: 'Temper Phase Distortion', dll: 'Temper.dll', cat: 'mod', catName: 'Фазовый дисторшн', desc: 'Уникальное фазовое искривление формы волны для футуристичного звука.', how: 'Динамический фазовый сдвиг с мягким овердрайвом.' },

  // --- Saturation, Drive & Guitar Amps ---
  { id: 'ivgi2', title: 'Klanghelm IVGI2', dll: 'IVGI2.dll', cat: 'drive', catName: 'Ламповая сатурация', desc: 'Мягкий аналоговый сатуратор. Делает голос близким, плотным и теплым.', how: 'Симметричное и асимметричное насыщение с частотно-зависимой динамикой.' },
  { id: 'varisaturator', title: 'Voxengo VariSaturator', dll: 'VariSaturator.dll', cat: 'drive', catName: 'Двухполосный сатуратор', desc: 'Раздельная сатурация низких и высоких частот лампами и транзисторами.', how: 'Двухполосный кроссовер с раздельными процессорами насыщения.' },
  { id: 'tapecrtiv', title: 'Voxengo CRTIV Tape Bus', dll: 'CRTIV Tape Bus.dll', cat: 'drive', catName: 'Магнитная лента', desc: 'Эмуляция студийного бобинного магнитофона с теплыми гармониками.', how: 'Моделирование компрессии магнитного слоя ленты и гистерезиса.' },
  { id: 'emissary', title: 'Ignite Amps Emissary', dll: 'Emissary.dll', cat: 'drive', catName: 'Ламповый усилитель', desc: 'Высококлассный ламповый гитарный усилитель (Clean & Lead).', how: 'Математическое моделирование ламп 12AX7 и 6L6/EL34.' },
  { id: 'nadir', title: 'Ignite Amps NadIR', dll: 'NadIR.dll', cat: 'drive', catName: 'Импульсный плеер (IR)', desc: 'Нулевая задержка конволюции импульсов кабинетов и микрофонов.', how: 'Прямая свертка FFT с аппаратной оптимизацией SIMD.' },
  { id: 'ts999', title: 'Ignite Amps TS-999', dll: 'TS-999.dll', cat: 'drive', catName: 'Tube Screamer Overdrive', desc: 'Эмуляция легендарной педали Ibanez Tube Screamer TS-9.', how: 'Срез низа и подъем средних частот перед диодным ограничением.' },
  { id: 'profet', title: 'Ignite Amps ProFET', dll: 'ProFET.dll', cat: 'drive', catName: 'JFET Предусилитель', desc: 'Агрессивный перегруз на полевых транзисторах JFET.', how: 'Каскадное моделирование полевых транзисторов.' },
  { id: 'bcfreeamp', title: 'Blue Cat Free Amp', dll: 'BC Free Amp VST.dll', cat: 'drive', catName: 'Усилитель Blue Cat', desc: '3 модели гитарных усилителей: Classic Clean, Classic Blue, Modern.', how: 'Аналоговое нелинейное моделирование.' },
  { id: 'rockrack', title: 'bx_rockrack V3 Player', dll: 'bx_rockrack V3 Player.dll', cat: 'drive', catName: 'Brainworx Rockrack', desc: 'Премиальный гитарный усилитель студийного класса от Brainworx.', how: 'Компонентное моделирование схемы Marshall и ENGL.' },
  { id: 'facebender', title: 'Face Bender', dll: 'Face Bender x64.dll', cat: 'drive', catName: 'Винтажный германиевый Fuzz', desc: 'Классический винтажный фузз на германиевых транзисторах.', how: 'Жесткое клиппирование с богатыми нечетными гармониками.' },
  { id: 'powershaper', title: 'Voxengo Powershaper', dll: 'Powershaper.dll', cat: 'drive', catName: 'Вейвшейпер', desc: 'Управляемое искажение формы волны.', how: 'Многоточечная табличная трансформация амплитуды.' },
  { id: 'lovend', title: 'Analog Obsession LOVEND', dll: 'LOVEND_x64.dll', cat: 'drive', catName: 'Низкочастотный сатуратор', desc: 'Добавляет суб-гармоники и плотный низ в бас и бочку.', how: 'Генерация второй гармоники в суб-басовом диапазоне.' },

  // --- Stereo & Mastering Utilities ---
  { id: 'wider', title: 'Polyverse Wider', dll: 'Wider.dll', cat: 'stereo', catName: 'Стерео расширитель', desc: '100% моносовместимый стерео-расширитель от Polyverse и Infected Mushroom.', how: 'Расширяет псевдостерео без фазовой деструкции в моно.' },
  { id: 'deewider', title: 'DOTEC-AUDIO DeeWider', dll: 'DeeWider.dll', cat: 'stereo', catName: 'Экспандер стерео-базы', desc: 'Мгновенное расширение стереопанорамы одной ручкой.', how: 'Мягкий фазовый сдвиг крайних частот.' },
  { id: 'a1stereo', title: 'A1StereoControl', dll: 'A1StereoControl-x64.dll', cat: 'stereo', catName: 'Стерео-контроллер', desc: 'Расширение стерео с безопасным басом (Safe Bass моно).', how: 'Оставляет низкие частоты строго по центру, расширяя только СЧ/ВЧ.' },
  { id: 'msed', title: 'Voxengo MSED', dll: 'MSED.dll', cat: 'stereo', catName: 'Mid-Side Кодировщик', desc: 'Раздельная регулировка центрального канала (Mid) и боковых (Side).', how: 'Матричное преобразование L/R в M/S и обратно.' },
  { id: 'stereotouch', title: 'Voxengo Stereo Touch', dll: 'Stereo Touch.dll', cat: 'stereo', catName: 'Моно в стерео', desc: 'Превращает монофонический сигнал микрофона в естественное стерео.', how: 'Фильтрованные полифонические задержки с противофазой.' },
  { id: 'upstereo', title: 'QuikQuak UpStereo', dll: 'UpStereo.dll', cat: 'stereo', catName: 'Стерео энхансер', desc: 'Стерео-экспандер с энхансером высоких частот и насыщением громкости.', how: 'Гармоническое стерео-расширение.' },
  { id: 'midsider', title: 'TheMidSider', dll: 'TheMidSider.dll', cat: 'stereo', catName: 'MS Пространство', desc: 'Простой Mid/Side регулятор баланса ширины сцены.', how: 'Прямое регулирование Side усиления.' },
  { id: 'ovc128', title: 'Voxengo OVC-128', dll: 'OVC-128.dll', cat: 'stereo', catName: '128x Клиппер', desc: 'Экстремально чистый мастеринговый клиппер со 128-кратным оверсэмплингом.', how: 'Полное устранение алиасинга при жестком клиппинге.' },
  { id: 'peakbuster', title: 'Voxengo Peakbuster', dll: 'Peakbuster.dll', cat: 'stereo', catName: 'Транзиент-бустер', desc: 'Возвращает жизнь и динамический панч сплющенным записям.', how: 'Усиление переходных процессов на основе энергии атаки.' },

  // --- Vocals, Pitch & Utilities ---
  { id: 'graillon', title: 'Auburn Sounds Graillon 2', dll: 'Auburn Sounds Graillon 2-64.dll', cat: 'vocal', catName: 'Автотюн и питч-корректор', desc: 'Профессиональный автотюн в реальном времени. Исправляет фальшивые ноты и делает T-Pain робот-эффект.', how: 'Мгновенный фазовый вокодер с привязкой к хроматической или выбранной тональности.' },
  { id: 'vocaldoubler', title: 'iZotope Vocal Doubler', dll: 'Vocal Doubler.dll', cat: 'vocal', catName: 'Дабл-трек вокала (ADT)', desc: 'Имитация записи второго дубля вокала для придания объема и ширины.', how: 'Рандомизированные микрозадержки и вариации питча левого и правого каналов.' },
  { id: 'chrissboost', title: 'Chris\'s Boost 2', dll: 'ChrissBoost_2.dll', cat: 'vocal', catName: 'Голосовой бустер громкости', desc: 'Легендарный компрессор-левелер из радиовещания для максимальной читаемости речи.', how: 'Скользящее окно среднего уровня громкости с мягким лимитированием.' },
  { id: 'bcgain', title: 'Blue Cat Gain 3 Stereo', dll: 'BC Gain 3 VST(Stereo).dll', cat: 'vocal', catName: 'Гейн / Триммер', desc: 'Идеально чистая регулировка громкости и баланса каналов.', how: 'Прямое 64-битное масштабирование от -60 до +60 dB.' },
  { id: 'blgain24', title: 'Blue Lab Gain 24', dll: 'BL-Gain24.dll', cat: 'vocal', catName: '24dB Предусилитель', desc: 'Точный предусилитель с шагом 0.1 dB.', how: 'Линейное умножение амплитуды.' },
  { id: 'blgain60', title: 'Blue Lab Gain 60', dll: 'BL-Gain60.dll', cat: 'vocal', catName: '60dB Предусилитель', desc: 'Высокочувствительный гейн для тихих динамических микрофонов.', how: 'Качественное усиление с защитой от переполнения.' },
  { id: 'reacontrolmidi', title: 'ReaControlMIDI', dll: 'reacontrolmidi-standalone.dll', cat: 'vocal', catName: 'MIDI Контроллер', desc: 'Управление плагинами и параметрами через MIDI команды.', how: 'Трансляция MIDI CC в параметры VST.' },
  { id: 'reafir', title: 'ReaFIR', dll: 'reafir_standalone.dll', cat: 'vocal', catName: 'Спектральное шумоподавление', desc: 'Адаптивное БПФ-шумоподавление без металлического призвука.', how: 'Спектральное вычитание постоянного профиля шума.' }
];

function createVstBlock(pluginId, customPath = '') {
  let meta = VST_PLUGINS_CATALOG[pluginId];
  if (!meta) {
    const found = ALL_VST_98_CATALOG.find(p => p.id === pluginId || (customPath && customPath.toLowerCase().includes(p.dll.toLowerCase())));
    if (found) {
      meta = {
        title: found.title,
        category: found.catName,
        dll: found.dll,
        desc: found.desc,
        howItWorks: found.how,
        params: [],
        presets: []
      };
    }
  }
  const params = {};
  if (meta && meta.params) {
    meta.params.forEach(p => {
      params[p.key] = p.default;
    });
  }
  return {
    id: nextId(),
    kind: 'vst',
    vstId: pluginId,
    title: meta ? meta.title : (customPath ? customPath.split(/[\\/]/).pop() : 'Custom VST Plugin'),
    library: customPath || (meta ? (VST_BASE_DIR + meta.dll) : ''),
    enabled: true,
    params: params
  };
}

function createRc505Block(effectId) {
  const meta = RC505_CATALOG[effectId];
  const params = {};
  if (meta && meta.params) {
    meta.params.forEach(p => {
      params[p.key] = p.default;
    });
  }
  return {
    id: nextId(),
    kind: 'rc505',
    effectId: effectId,
    title: meta ? meta.title : `BOSS RC-505 Effect: ${effectId}`,
    enabled: true,
    params: params
  };
}

const state = {
  instantMode: localStorage.getItem('resona_instant_mode') !== 'false',
  activeTabId: (savedTabs && savedActiveTabId && savedTabs.some(t => t.id === savedActiveTabId)) ? savedActiveTabId : ((savedTabs && savedTabs[0]) ? savedTabs[0].id : 'tab_config'),
  tabs: savedTabs || [
    {
      id: 'tab_config',
      name: 'config.txt',
      selectedDevice: localStorage.getItem('resona_selected_device') || 'all',
      selectedChannel: localStorage.getItem('resona_selected_channel') || 'all',
      blocks: createDefaultBlocks()
    }
  ],
  draggingNode: null
};

// Helper: Active Tab
function getActiveTab() {
  let tab = state.tabs.find(t => t.id === state.activeTabId);
  if (!tab) {
    tab = state.tabs[0];
    if (tab) state.activeTabId = tab.id;
  }
  return tab;
}

// Clipboard & History
let clipboardBlock = null;
let undoStack = [];
let redoStack = [];
let isUndoRedoing = false;

// DOM References
const configTabBar = document.getElementById('configTabBar');
const blocksContainer = document.getElementById('blocksContainer');
const canvas = document.getElementById('analysisCanvas');
const ctx = canvas.getContext('2d');
const clipStatusBox = document.getElementById('clipStatusBox');
const clipStatusText = document.getElementById('clipStatusText');
const peakGainText = document.getElementById('peakGainText');
const analysisTimeText = document.getElementById('analysisTimeText');
const curveTooltip = document.getElementById('curveTooltip');
const addBlockBtn = document.getElementById('addBlockBtn');
const addMenuDropdown = document.getElementById('addMenuDropdown');
const rawConfigModal = document.getElementById('rawConfigModal');
const rawConfigText = document.getElementById('rawConfigText');
const configuratorModal = document.getElementById('configuratorModal');
const cfgTableBody = document.getElementById('cfgTableBody');
const cfgTroubleCheck = document.getElementById('cfgTroubleCheck');
const cfgTroubleBox = document.getElementById('cfgTroubleBox');
const togglePanelBtn = document.getElementById('togglePanelBtn');
const analysisPanel = document.getElementById('analysisPanel');
const newConfigModal = document.getElementById('newConfigModal');
const saveAsModal = document.getElementById('saveAsModal');
const aboutModal = document.getElementById('aboutModal');
const wizardModal = document.getElementById('wizardModal');

// --- Window Resize & Observer for Responsive Windowed Mode (Task 8) ---
function resizeAnalysisCanvas() {
  const rect = canvas.getBoundingClientRect();
  if (rect.width > 0 && rect.height > 0) {
    if (canvas.width !== Math.floor(rect.width) || canvas.height !== Math.floor(rect.height)) {
      canvas.width = Math.floor(rect.width);
      canvas.height = Math.floor(rect.height);
    }
    renderAnalysis();
  }
}
window.addEventListener('resize', resizeAnalysisCanvas);

const canvasWrapEl = document.querySelector('.analysis-canvas-wrap');
if (window.ResizeObserver && canvasWrapEl) {
  new ResizeObserver(() => {
    resizeAnalysisCanvas();
  }).observe(canvasWrapEl);
}

// --- Biquad Audio Math ---
function evaluateBiquadDb(type, fc, gainDb, q, freqHz) {
  const w0 = 2.0 * Math.PI * fc / SAMPLE_RATE;
  const cosW0 = Math.cos(w0);
  const sinW0 = Math.sin(w0);
  const alpha = sinW0 / (2.0 * Math.max(q, 0.01));
  const A = Math.pow(10.0, gainDb / 40.0);

  let b0, b1, b2, a0, a1, a2;
  switch (type) {
    case 'PK':
      b0 = 1.0 + alpha * A;
      b1 = -2.0 * cosW0;
      b2 = 1.0 - alpha * A;
      a0 = 1.0 + alpha / A;
      a1 = -2.0 * cosW0;
      a2 = 1.0 - alpha / A;
      break;
    case 'LS': {
      const s = 2.0 * Math.sqrt(A) * alpha;
      b0 = A * ((A + 1.0) - (A - 1.0) * cosW0 + s);
      b1 = 2.0 * A * ((A - 1.0) - (A + 1.0) * cosW0);
      b2 = A * ((A + 1.0) - (A - 1.0) * cosW0 - s);
      a0 = (A + 1.0) + (A - 1.0) * cosW0 + s;
      a1 = -2.0 * ((A - 1.0) + (A + 1.0) * cosW0);
      a2 = (A + 1.0) - (A - 1.0) * cosW0 - s;
      break;
    }
    case 'HS': {
      const s = 2.0 * Math.sqrt(A) * alpha;
      b0 = A * ((A + 1.0) + (A - 1.0) * cosW0 + s);
      b1 = -2.0 * A * ((A - 1.0) + (A + 1.0) * cosW0);
      b2 = A * ((A + 1.0) - (A - 1.0) * cosW0 - s);
      a0 = (A + 1.0) - (A - 1.0) * cosW0 + s;
      a1 = 2.0 * ((A - 1.0) - (A + 1.0) * cosW0);
      a2 = (A + 1.0) - (A - 1.0) * cosW0 - s;
      break;
    }
    case 'LP':
      b0 = (1.0 - cosW0) / 2.0;
      b1 = 1.0 - cosW0;
      b2 = (1.0 - cosW0) / 2.0;
      a0 = 1.0 + alpha;
      a1 = -2.0 * cosW0;
      a2 = 1.0 - alpha;
      break;
    case 'HP':
      b0 = (1.0 + cosW0) / 2.0;
      b1 = -(1.0 + cosW0);
      b2 = (1.0 + cosW0) / 2.0;
      a0 = 1.0 + alpha;
      a1 = -2.0 * cosW0;
      a2 = 1.0 - alpha;
      break;
    case 'NO':
      b0 = 1.0;
      b1 = -2.0 * cosW0;
      b2 = 1.0;
      a0 = 1.0 + alpha;
      a1 = -2.0 * cosW0;
      a2 = 1.0 - alpha;
      break;
    default:
      return 0.0;
  }

  b0 /= a0; b1 /= a0; b2 /= a0; a1 /= a0; a2 /= a0;

  const w = 2.0 * Math.PI * freqHz / SAMPLE_RATE;
  const cosW = Math.cos(w);
  const sinW = Math.sin(w);
  const cos2W = Math.cos(2.0 * w);
  const sin2W = Math.sin(2.0 * w);

  const numRe = b0 + b1 * cosW + b2 * cos2W;
  const numIm = -b1 * sinW - b2 * sin2W;
  const denRe = 1.0 + a1 * cosW + a2 * cos2W;
  const denIm = -a1 * sinW - a2 * sin2W;

  const magLin = Math.sqrt((numRe * numRe + numIm * numIm) / Math.max(denRe * denRe + denIm * denIm, 1e-12));
  return 20.0 * Math.log10(Math.max(magLin, 1e-6));
}

// Logarithmic Coordinates
const MIN_F = 10;
const MAX_F = 20000;
const MIN_DB = -30;
const MAX_DB = 25;

function fToX(f, w) {
  const minL = Math.log10(MIN_F);
  const maxL = Math.log10(MAX_F);
  return ((Math.log10(f) - minL) / (maxL - minL)) * w;
}

function xToF(x, w) {
  const minL = Math.log10(MIN_F);
  const maxL = Math.log10(MAX_F);
  const r = Math.max(0, Math.min(1, x / w));
  return Math.pow(10, minL + r * (maxL - minL));
}

function dbToY(db, h) {
  const r = (db - MIN_DB) / (MAX_DB - MIN_DB);
  return h - (r * h);
}

function yToDb(y, h) {
  const r = 1.0 - (y / h);
  return MIN_DB + r * (MAX_DB - MIN_DB);
}

function updateWindowTitle() {
  const tab = getActiveTab();
  if (tab && tab.name) {
    document.title = `Configuration Editor - Equalizer FD CRAFT [${tab.name}]`;
  } else {
    document.title = 'Configuration Editor - Equalizer FD CRAFT';
  }
}

// --- Render Multi-Tab Bar (Task 10) ---
function renderTabs() {
  if (!configTabBar) return;
  updateWindowTitle();
  configTabBar.innerHTML = '';

  state.tabs.forEach(tab => {
    const isActive = tab.id === state.activeTabId;
    const tabEl = document.createElement('div');
    tabEl.className = `tab-item ${isActive ? 'active' : ''}`;
    tabEl.dataset.tabId = tab.id;

    tabEl.innerHTML = `
      <span>${tab.name}</span>
      ${state.tabs.length > 1 ? `<span class="tab-close" data-close-id="${tab.id}" title="Закрыть вкладку">&times;</span>` : ''}
    `;

    tabEl.addEventListener('click', (e) => {
      if (e.target.dataset.closeId) {
        closeTab(e.target.dataset.closeId);
        return;
      }
      switchTab(tab.id);
    });

    configTabBar.appendChild(tabEl);
  });

  // Plus button in tab bar
  const addTabBtn = document.createElement('button');
  addTabBtn.className = 'tab-new-btn';
  addTabBtn.title = 'Создать новую вкладку пресета (Ctrl+N)';
  addTabBtn.textContent = '+';
  addTabBtn.addEventListener('click', openNewConfigModal);
  configTabBar.appendChild(addTabBtn);
}

function switchTab(tabId) {
  state.activeTabId = tabId;
  const tab = getActiveTab();
  if (tab) {
    const devSel = document.getElementById('deviceSelector');
    if (devSel && tab.selectedDevice) setDeviceSelectorValue(devSel, tab.selectedDevice);

    const chSel = document.getElementById('channelSelector');
    if (chSel && tab.selectedChannel) chSel.value = tab.selectedChannel;

    if (!tab.blocks || tab.blocks.length === 0) {
      tab.blocks = createDefaultBlocks();
    }
  }
  renderTabs();
  renderBlocks();
  updateAll(false);
}

function closeTab(tabId) {
  if (state.tabs.length <= 1) return;
  state.tabs = state.tabs.filter(t => t.id !== tabId);
  if (state.activeTabId === tabId) {
    state.activeTabId = state.tabs[0].id;
  }
  saveTabsState();
  switchTab(state.activeTabId);
}

function saveTabsState() {
  localStorage.setItem('resona_tabs_state', JSON.stringify(state.tabs));
}

// --- Render Analysis Panel Graph ---
function renderAnalysis() {
  const t0 = performance.now();
  const w = canvas.width || 1280;
  const h = canvas.height || 230;

  ctx.clearRect(0, 0, w, h);

  // 1. Dark scope background
  ctx.fillStyle = '#14171a';
  ctx.fillRect(0, 0, w, h);

  // 2. Frequency Grid (Logarithmic)
  const freqs = [10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
  ctx.strokeStyle = '#2d333b';
  ctx.lineWidth = 1;
  ctx.fillStyle = '#9ca3af';
  ctx.font = '10px "Segoe UI", Tahoma, sans-serif';

  freqs.forEach(f => {
    const x = fToX(f, w);
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, h);
    ctx.stroke();

    const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
    ctx.fillText(label, x + 3, h - 6);
  });

  // 3. Decibel Grid
  const dbs = [20, 10, 0, -10, -20];
  dbs.forEach(db => {
    const y = dbToY(db, h);
    ctx.strokeStyle = db === 0 ? '#0284c7' : '#262c34';
    ctx.lineWidth = db === 0 ? 1.8 : 1;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
    ctx.stroke();

    ctx.fillStyle = db === 0 ? '#38bdf8' : '#9ca3af';
    ctx.fillText(`${db > 0 ? '+' : ''}${db} dB`, 6, y - 3);
  });

  // 4. Calculate Total Magnitude Response Curve for active tab
  const tab = getActiveTab();
  const currentBlocks = tab ? tab.blocks : [];

  let peakDb = -999;
  let peakFreq = 100;

  ctx.strokeStyle = '#00d2ff'; // Cyan
  ctx.lineWidth = 2.0;
  ctx.beginPath();

  for (let x = 0; x <= w; x += 2) {
    const f = xToF(x, w);
    let totalDb = 0.0;

    currentBlocks.forEach(b => {
      if (!b.enabled) return;
      if (b.kind === 'preamp') {
        totalDb += b.gain;
      } else if (b.kind === 'parametric') {
        b.filters.forEach(flt => {
          if (flt.enabled) {
            totalDb += evaluateBiquadDb(flt.type, flt.fc, flt.gain, flt.q, f);
          }
        });
      } else if (b.kind === 'graphiceq15' || b.kind === 'graphiceq31') {
        const bandwidth = b.kind === 'graphiceq31' ? 0.4 : 0.8;
        b.bands.forEach(band => {
          const octDist = Math.abs(Math.log2(f / band.f));
          if (octDist < bandwidth) {
            totalDb += band.gain * (1.0 - (octDist / bandwidth));
          }
        });
      }
    });

    if (totalDb > peakDb) {
      peakDb = totalDb;
      peakFreq = f;
    }

    const y = dbToY(totalDb, h);
    if (x === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();

  // Draw interactive draggable nodes for parametric filters
  currentBlocks.forEach(b => {
    if (b.kind === 'parametric' && b.enabled) {
      b.filters.forEach(flt => {
        if (!flt.enabled) return;
        const x = fToX(flt.fc, w);
        const y = dbToY(flt.gain, h);

        ctx.fillStyle = '#ffffff';
        ctx.strokeStyle = '#0078d7';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(x, y, 5, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = '#00d2ff';
        ctx.font = '10px monospace';
        ctx.fillText(`F${flt.id % 100}`, x - 8, y - 8);
      });
    }
  });

  // Update Status Badges (Zero Emojis)
  const elapsed = (performance.now() - t0).toFixed(1);
  analysisTimeText.textContent = window.t
    ? window.t('analysis.calcTime', { ms: elapsed }, `Расчет: ${elapsed} мс`)
    : `Расчет: ${elapsed} мс`;

  peakGainText.textContent = window.t
    ? window.t('analysis.peakAt', { gain: (peakDb > 0 ? '+' : '') + peakDb.toFixed(2), freq: Math.round(peakFreq) }, `Пик: ${peakDb > 0 ? '+' : ''}${peakDb.toFixed(2)} dB на ${Math.round(peakFreq)} Hz`)
    : `Пик: ${peakDb > 0 ? '+' : ''}${peakDb.toFixed(2)} dB на ${Math.round(peakFreq)} Hz`;

  if (peakDb > 0.0) {
    clipStatusBox.className = 'status-indicator-clean clip';
    clipStatusText.textContent = window.t
      ? window.t('toolbar.clipOverload', { val: peakDb.toFixed(2) }, `Клиппинг: +${peakDb.toFixed(2)} dB`)
      : `Клиппинг: +${peakDb.toFixed(2)} dB`;
  } else {
    clipStatusBox.className = 'status-indicator-clean ok';
    clipStatusText.textContent = window.t
      ? window.t('toolbar.clipOk', { val: peakDb.toFixed(1) }, `Нет клиппинга (Пик: ${peakDb.toFixed(1)} dB)`)
      : `Нет клиппинга (Пик: ${peakDb.toFixed(1)} dB)`;
  }
}


// --- Localization Helpers for Filters & Plugins ---
function getFilterHint(type) {
  if (window.t) {
    const hint = window.t('blocks.parametric.hints.' + type);
    if (hint && typeof hint === 'string' && hint !== 'blocks.parametric.hints.' + type) return hint;
  }
  return FILTER_HINTS[type] || '';
}

function getFilterTypeName(type) {
  if (window.t) {
    const name = window.t('blocks.parametric.types.' + type);
    if (name && typeof name === 'string' && name !== 'blocks.parametric.types.' + type) return name;
  }
  switch (type) {
    case 'PK': return 'Параметрический (Колокол / Peaking)';
    case 'LS': return 'НЧ полка (Low-shelf)';
    case 'HS': return 'ВЧ полка (High-shelf)';
    case 'LP': return 'Срез ВЧ (Low-pass / ФНЧ)';
    case 'HP': return 'Срез НЧ (High-pass / ФВЧ)';
    case 'NO': return 'Вырез помехи/гула (Notch)';
    default: return type;
  }
}

function getVstLocalizedMeta(vstId, libraryPath) {
  const dict = window.t ? window.t('vstPlugins.' + vstId) : null;
  const isDictValid = dict && typeof dict === 'object' && dict.title;

  let baseMeta = VST_PLUGINS_CATALOG[vstId];
  if (!baseMeta && Array.isArray(ALL_VST_98_CATALOG)) {
    const found = ALL_VST_98_CATALOG.find(p => p.id === vstId || (libraryPath && libraryPath.toLowerCase().includes(p.dll.toLowerCase())));
    if (found) {
      baseMeta = {
        id: found.id,
        title: found.title,
        category: found.catName,
        desc: found.desc,
        howItWorks: found.how,
        params: [],
        presets: []
      };
    }
  }

  const customTitle = window.t ? window.t('blocks.vst.customTitle', null, 'Пользовательский VST плагин') : 'Пользовательский VST плагин';
  const customCat = window.t ? window.t('blocks.vst.customCategory', null, 'VST Плагин') : 'VST Плагин';
  const customDesc = window.t ? window.t('blocks.vst.customDesc', null, 'Сторонний плагин в формате VST 2.4 DLL для обработки звука в тракте Equalizer FD CRAFT.') : 'Сторонний плагин в формате VST 2.4 DLL для обработки звука в тракте Equalizer FD CRAFT.';
  const customHow = window.t ? window.t('blocks.vst.customHow', null, 'Загружается напрямую в аудиодрайвер audiodg.exe и обрабатывает аудиопоток с нулевой задержкой.') : 'Загружается напрямую в аудиодрайвер audiodg.exe и обрабатывает аудиопоток с нулевой задержкой.';

  if (isDictValid) {
    return {
      title: dict.title || (baseMeta ? baseMeta.title : customTitle),
      category: dict.category || (baseMeta ? baseMeta.category : customCat),
      desc: dict.desc || (baseMeta ? baseMeta.desc : customDesc),
      howItWorks: dict.how || (baseMeta ? (baseMeta.howItWorks || baseMeta.how) : customHow),
      params: (baseMeta && baseMeta.params) ? baseMeta.params.map(p => ({
        ...p,
        label: (dict.params && dict.params[p.key]) || p.label
      })) : [],
      presets: (baseMeta && baseMeta.presets) ? baseMeta.presets.map((pr, idx) => ({
        ...pr,
        name: (dict.presets && dict.presets[String(idx)]) || pr.name
      })) : []
    };
  }

  if (baseMeta) {
    return {
      ...baseMeta,
      howItWorks: baseMeta.howItWorks || baseMeta.how || customHow
    };
  }

  return {
    title: customTitle,
    category: customCat,
    desc: customDesc,
    howItWorks: customHow,
    params: [],
    presets: []
  };
}

function getRc505LocalizedMeta(effectId) {
  const dict = window.t ? window.t('rc505Plugins.' + effectId) : null;
  const isDictValid = dict && typeof dict === 'object' && dict.title;
  const baseMeta = RC505_CATALOG[effectId];

  if (isDictValid) {
    return {
      title: dict.title || (baseMeta ? baseMeta.title : effectId),
      category: dict.category || (baseMeta ? baseMeta.category : 'BOSS RC-505 Mk2 DSP'),
      desc: dict.desc || (baseMeta ? baseMeta.desc : ''),
      howItWorks: dict.how || (baseMeta ? baseMeta.howItWorks : ''),
      params: (baseMeta && baseMeta.params) ? baseMeta.params.map(p => ({
        ...p,
        label: (dict.params && dict.params[p.key]) || p.label
      })) : [],
      presets: (baseMeta && baseMeta.presets) ? baseMeta.presets.map((pr, idx) => ({
        ...pr,
        name: (dict.presets && dict.presets[String(idx)]) || pr.name
      })) : []
    };
  }

  if (baseMeta) {
    return baseMeta;
  }

  return {
    title: 'BOSS RC-505 Mk2 Effect',
    category: 'BOSS RC-505 Mk2',
    desc: 'Roland BOSS RC-505 Mk2 DSP effect.',
    howItWorks: 'Real-time hardware DSP emulation.',
    params: [],
    presets: []
  };
}

// Localized Block Title Resolver
function getBlockLocalizedTitle(block) {
  if (!window.t) return block.title;
  switch (block.kind) {
    case 'preamp': return window.t('blocks.preamp.title', null, window.t('add.preamp', null, block.title));
    case 'parametric': return window.t('blocks.parametric.title', null, window.t('add.catParametric', null, block.title));
    case 'graphiceq15': return window.t('add.geq15', null, block.title);
    case 'graphiceq31': return window.t('add.geq31', null, block.title);
    case 'device': return window.t('add.device', null, block.title);
    case 'channel': return window.t('add.channel', null, block.title);
    case 'include': return window.t('blocks.include.title', null, window.t('add.include', null, block.title));
    case 'vst': return getVstLocalizedMeta(block.vstId, block.library).title;
    case 'rc505': return getRc505LocalizedMeta(block.effectId).title;
    case 'delay': return window.t('add.delay', null, block.title);
    case 'copy': return window.t('add.copy', null, block.title);
    default: return block.title;
  }
}

// --- Render Filter Blocks Stack (Guarantees non-empty display) ---
function renderBlocks(targetScrollBlockId = null) {
  const scrollContainer = document.getElementById('blocksWorkspace');
  const prevScrollTop = scrollContainer ? scrollContainer.scrollTop : 0;

  blocksContainer.innerHTML = '';
  const tab = getActiveTab();
  if (!tab) return;

  const isEn = window.I18N && window.I18N.lang === 'en';

  // Windowed mode guarantee: if blocks are empty, render friendly guided placeholder
  if (!tab.blocks || tab.blocks.length === 0) {
    const emptyBox = document.createElement('div');
    emptyBox.className = 'blocks-empty-state';
    const emptyTitle = window.t ? window.t('blocks.emptyTitle', null, isEn ? 'Configuration is currently empty' : 'Конфигурация пока пуста') : 'Конфигурация пока пуста';
    const emptyDesc = window.t ? window.t('blocks.emptyDesc', null, isEn ? 'Click the green "+" button below or choose a ready preset (for stream, podcast, or Discord) via "File" &rarr; "New..."' : 'Нажмите зеленую кнопку «+» внизу или выберите готовый пресет (для стрима, подкаста или Discord) через меню «Файл» &rarr; «Создать...»') : 'Нажмите зеленую кнопку «+» внизу или выберите готовый пресет через меню «Файл»';
    const emptyBtn = window.t ? window.t('blocks.emptyBtn', null, isEn ? '+ Add Default Equalizer' : '+ Добавить базовый эквалайзер') : '+ Добавить базовый эквалайзер';
    emptyBox.innerHTML = `
      <div class="empty-state-title">${emptyTitle}</div>
      <div class="empty-state-desc">${emptyDesc}</div>
      <button class="win-btn primary" id="btnQuickAddDefaultEq" style="margin-top: 6px; padding: 4px 14px;">${emptyBtn}</button>
    `;
    blocksContainer.appendChild(emptyBox);
    emptyBox.querySelector('#btnQuickAddDefaultEq')?.addEventListener('click', () => {
      tab.blocks = createDefaultBlocks();
      renderBlocks();
      updateAll();
    });
    return;
  }

  tab.blocks.forEach((block, index) => {
    const card = document.createElement('div');
    card.className = 'filter-block-card';
    card.dataset.blockId = block.id;

    // Header bar
    const header = document.createElement('div');
    header.className = 'filter-block-header';

    const blockTitle = getBlockLocalizedTitle(block);
    const powerTitle = block.enabled
      ? (isEn ? 'Enabled (click to bypass)' : 'Включено (нажмите для выключения)')
      : (isEn ? 'Bypassed (click to enable)' : 'Выключено (нажмите для включения)');
    const moveUpTitle = isEn ? 'Move up' : 'Переместить выше';
    const moveDownTitle = isEn ? 'Move down' : 'Переместить ниже';
    const deleteTitle = isEn ? 'Delete filter block' : 'Удалить блок фильтра';

    header.innerHTML = `
      <div class="block-left">
        <button class="power-btn ${block.enabled ? '' : 'off'}" data-block-id="${block.id}" title="${powerTitle}">
          ${renderPowerIcon(11)}
        </button>
        <span class="block-title-text">${index + 1}: ${blockTitle}</span>
      </div>
      <div class="block-right">
        <button class="btn-icon-sm move-up" data-idx="${index}" title="${moveUpTitle}">&#x25B2;</button>
        <button class="btn-icon-sm move-down" data-idx="${index}" title="${moveDownTitle}">&#x25BC;</button>
        <button class="btn-icon-sm delete" data-block-id="${block.id}" title="${deleteTitle}">&times;</button>
      </div>
    `;
    card.appendChild(header);

    // Body content
    const content = document.createElement('div');
    content.className = 'filter-block-content';

    if (block.kind === 'preamp') {
      const preampLbl = window.t ? window.t('blocks.preamp.label', null, 'Предусиление:') : 'Предусиление:';
      content.innerHTML = `
        <div class="preamp-row">
          <span class="preamp-label">${preampLbl}</span>
          <input type="range" class="win-slider preamp-slider" min="-30" max="20" step="0.25" value="${block.gain}" data-block-id="${block.id}">
          <input type="text" class="win-input-num preamp-input" value="${block.gain >= 0 ? '+' : ''}${block.gain.toFixed(2)} dB" data-block-id="${block.id}">
        </div>
      `;
    } else if (block.kind === 'delay') {
      const delayLbl = window.t ? window.t('blocks.delay.label', null, 'Задержка:') : 'Задержка:';
      const delayVal = typeof block.delay === 'number' ? block.delay : 10.0;
      content.innerHTML = `
        <div class="preamp-row">
          <span class="preamp-label">${delayLbl}</span>
          <input type="range" class="win-slider delay-slider" min="0" max="500" step="0.5" value="${delayVal}" data-block-id="${block.id}">
          <input type="text" class="win-input-num delay-input" value="${delayVal.toFixed(2)} ms" data-block-id="${block.id}">
        </div>
      `;
    } else if (block.kind === 'include') {
      const fileLbl = window.t ? window.t('blocks.include.fileLabel', null, 'Файл конфигурации:') : 'Файл конфигурации:';
      const filePlaceholder = window.t ? window.t('blocks.include.placeholder', null, 'например: example.txt или mic_preset.txt') : 'например: example.txt или mic_preset.txt';
      const btnBrowseTxt = window.t ? window.t('blocks.include.btnBrowse', null, 'Выбрать...') : 'Выбрать...';
      const btnOpenTxt = window.t ? window.t('blocks.include.btnOpen', null, 'Открыть') : 'Открыть';
      content.innerHTML = `
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 11px; font-weight: 600; color: #475569;">${fileLbl}</span>
          <input type="text" class="win-select include-path-input" style="flex: 1; font-family: Consolas, monospace; font-size: 11px;" value="${block.path || ''}" data-block-id="${block.id}" placeholder="${filePlaceholder}">
          <button class="win-btn btn-include-browse" data-block-id="${block.id}">${btnBrowseTxt}</button>
          <button class="win-btn btn-include-open" data-block-id="${block.id}">${btnOpenTxt}</button>
        </div>
      `;
    } else if (block.kind === 'parametric') {
      const thPower = window.t ? window.t('blocks.parametric.thPower', null, 'Вкл') : 'Вкл';
      const thType = window.t ? window.t('blocks.parametric.thType', null, 'Тип фильтра') : 'Тип фильтра';
      const thFc = window.t ? window.t('blocks.parametric.thFc', null, 'Частота') : 'Частота';
      const thGain = window.t ? window.t('blocks.parametric.thGain', null, 'Усиление') : 'Усиление';
      const thQ = window.t ? window.t('blocks.parametric.thQ', null, 'Добротность (Q)') : 'Добротность (Q)';
      const thDelete = window.t ? window.t('blocks.parametric.thDelete', null, 'Удалить') : 'Удалить';
      const btnAddBand = window.t ? window.t('blocks.parametric.btnAddBand', null, '+ Добавить полосу фильтра') : '+ Добавить полосу фильтра';
      const unitHz = window.t ? window.t('analysis.unitHz', null, 'Гц') : 'Гц';
      const unitDb = window.t ? window.t('analysis.unitDb', null, 'дБ') : 'дБ';

      let tableHtml = `
        <table class="param-table">
          <thead>
            <tr>
              <th width="36" style="text-align: center;">${thPower}</th>
              <th>${thType}</th>
              <th>${thFc}</th>
              <th>${thGain}</th>
              <th>${thQ}</th>
              <th width="36" style="text-align: center;">${thDelete}</th>
            </tr>
          </thead>
          <tbody>
      `;
      block.filters.forEach((f) => {
        const hint = getFilterHint(f.type);
        const fltPowerTitle = f.enabled ? (window.t ? window.t('blocks.parametric.powerOn', null, 'Фильтр активен') : 'Фильтр активен') : (window.t ? window.t('blocks.parametric.powerOff', null, 'Фильтр выключен (Bypass)') : 'Фильтр выключен (Bypass)');
        const fltTitleFc = window.t ? window.t('blocks.parametric.titleFc', null, 'Центральная частота обработки в Герцах') : 'Центральная частота обработки в Герцах';
        const fltTitleGain = window.t ? window.t('blocks.parametric.titleGain', null, 'Подъем (+) или ослабление (-) частоты в децибелах') : 'Подъем (+) или ослабление (-) частоты в децибелах';
        const fltTitleQ = window.t ? window.t('blocks.parametric.titleQ', null, 'Добротность / ширина полосы действия фильтра') : 'Добротность / ширина полосы действия фильтра';
        const fltTitleDel = window.t ? window.t('blocks.parametric.titleDelete', null, 'Удалить эту полосу фильтра') : 'Удалить эту полосу фильтра';

        tableHtml += `
          <tr>
            <td class="cell-center">
              <button class="power-btn ${f.enabled ? '' : 'off'}" data-parent="${block.id}" data-flt-id="${f.id}" title="${fltPowerTitle}">
                ${renderPowerIcon(9)}
              </button>
            </td>
            <td>
              <div style="display: flex; align-items: center; gap: 4px;">
                <select class="win-select flt-type-sel" data-parent="${block.id}" data-flt-id="${f.id}" style="width: 230px;" title="${hint}">
                  <option value="PK" ${f.type === 'PK' ? 'selected' : ''} title="${getFilterHint('PK')}">${getFilterTypeName('PK')}</option>
                  <option value="LS" ${f.type === 'LS' ? 'selected' : ''} title="${getFilterHint('LS')}">${getFilterTypeName('LS')}</option>
                  <option value="HS" ${f.type === 'HS' ? 'selected' : ''} title="${getFilterHint('HS')}">${getFilterTypeName('HS')}</option>
                  <option value="LP" ${f.type === 'LP' ? 'selected' : ''} title="${getFilterHint('LP')}">${getFilterTypeName('LP')}</option>
                  <option value="HP" ${f.type === 'HP' ? 'selected' : ''} title="${getFilterHint('HP')}">${getFilterTypeName('HP')}</option>
                  <option value="NO" ${f.type === 'NO' ? 'selected' : ''} title="${getFilterHint('NO')}">${getFilterTypeName('NO')}</option>
                </select>
                <span class="flt-info-badge" title="${hint}">?</span>
              </div>
              <div class="flt-hint-text">${hint}</div>
            </td>
            <td><input type="number" class="win-input-num flt-fc" data-parent="${block.id}" data-flt-id="${f.id}" value="${Math.round(f.fc)}" min="20" max="20000" title="${fltTitleFc}"> ${unitHz}</td>
            <td><input type="number" step="0.5" class="win-input-num flt-gain" data-parent="${block.id}" data-flt-id="${f.id}" value="${f.gain.toFixed(1)}" title="${fltTitleGain}"> ${unitDb}</td>
            <td><input type="number" step="0.05" class="win-input-num flt-q" data-parent="${block.id}" data-flt-id="${f.id}" value="${f.q.toFixed(2)}" min="0.1" max="20" title="${fltTitleQ}"></td>
            <td class="cell-center">
              <button class="flt-del-btn" data-parent="${block.id}" data-flt-id="${f.id}" title="${fltTitleDel}">&times;</button>
            </td>
          </tr>
        `;
      });
      tableHtml += `
          </tbody>
        </table>
        <div style="margin-top: 8px;">
          <button class="win-btn add-sub-flt-btn" data-parent="${block.id}">${btnAddBand}</button>
        </div>
      `;
      content.innerHTML = tableHtml;
    } else if (block.kind === 'graphiceq15' || block.kind === 'graphiceq31') {
      const is31 = block.kind === 'graphiceq31';
      let ghtml = `<div class="graphic-eq-bank ${is31 ? 'geq-31' : ''}">`;
      block.bands.forEach((b, bi) => {
        let label = b.f >= 1000 ? (b.f % 1000 === 0 ? (b.f / 1000) + 'k' : (b.f / 1000).toFixed(1) + 'k') : b.f;
        ghtml += `
          <div class="geq-slider-col">
            <span class="geq-db-val">${b.gain > 0 ? '+' : ''}${b.gain}</span>
            <input type="range" class="geq-v-slider" min="-15" max="15" step="1" value="${b.gain}" data-block-id="${block.id}" data-band-idx="${bi}">
            <span class="geq-freq-label">${label}</span>
          </div>
        `;
      });
      ghtml += '</div>';
      content.innerHTML = ghtml;
    } else if (block.kind === 'vst') {
      const meta = getVstLocalizedMeta(block.vstId, block.library);
      const purposeLbl = window.t ? window.t('blocks.vst.purposeLabel', null, 'Для чего нужен:') : 'Для чего нужен:';
      const howLbl = window.t ? window.t('blocks.vst.howLabel', null, 'Как работает:') : 'Как работает:';
      const dllLbl = window.t ? window.t('blocks.vst.dllLabel', null, 'DLL библиотека:') : 'DLL библиотека:';
      const presetsLbl = window.t ? window.t('blocks.vst.presetsLabel', null, 'Быстрые пресеты:') : 'Быстрые пресеты:';

      let vhtml = `
        <div class="plugin-desc-banner">
          <div style="font-weight: 700; color: #166534; margin-bottom: 2px;">${meta.title} — ${meta.category}</div>
          <div style="margin-bottom: 4px;"><strong>${purposeLbl}</strong> ${meta.desc}</div>
          <div style="color: #475569;"><strong>${howLbl}</strong> ${meta.howItWorks}</div>
        </div>

        <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 10px;">
          <span style="font-size: 11px; font-weight: 600; color: #475569;">${dllLbl}</span>
          <input type="text" class="win-select vst-lib-path" style="flex: 1; font-family: Consolas, monospace; font-size: 11px;" value="${block.library || ''}" data-block-id="${block.id}" title="Путь к 64-битному .dll файлу VST плагина">
        </div>
      `;

      if (meta.params && meta.params.length > 0) {
        vhtml += `<div class="param-grid">`;
        meta.params.forEach(p => {
          const curVal = (block.params && block.params[p.key] !== undefined) ? block.params[p.key] : p.default;
          vhtml += `
            <div class="param-item">
              <div class="param-item-header">
                <span class="param-name">${p.label}</span>
                <span class="param-val-badge" id="badge_${block.id}_${p.key}">${curVal} ${p.unit}</span>
              </div>
              <div class="param-slider-row">
                <input type="range" class="param-slider vst-slider" min="${p.min}" max="${p.max}" step="${p.step}" value="${curVal}" data-block-id="${block.id}" data-param-key="${p.key}" data-unit="${p.unit}">
              </div>
            </div>
          `;
        });
        vhtml += `</div>`;
      }

      if (meta.presets && meta.presets.length > 0) {
        vhtml += `
          <div class="preset-pills-row">
            <span class="preset-pill-label">${presetsLbl}</span>
        `;
        meta.presets.forEach((pr, pi) => {
          vhtml += `
            <button class="pill-btn vst-preset-pill" data-block-id="${block.id}" data-vst-id="${block.vstId}" data-preset-idx="${pi}">
              ${pr.name}
            </button>
          `;
        });
        vhtml += `</div>`;
      }

      content.innerHTML = vhtml;
    } else if (block.kind === 'rc505') {
      const meta = getRc505LocalizedMeta(block.effectId);
      const purposeLbl = window.t ? window.t('blocks.rc505.purposeLabel', null, 'Для чего нужен:') : 'Для чего нужен:';
      const howLbl = window.t ? window.t('blocks.rc505.howLabel', null, 'Как работает:') : 'Как работает:';
      const presetsLbl = window.t ? window.t('blocks.rc505.presetsLabel', null, 'Студийные пресеты Roland BOSS:') : 'Студийные пресеты Roland BOSS:';

      let rhtml = `
        <div class="plugin-desc-banner rc505-desc">
          <div style="font-weight: 700; color: #1e40af; margin-bottom: 2px;">${meta.title} — ${meta.category}</div>
          <div style="margin-bottom: 4px;"><strong>${purposeLbl}</strong> ${meta.desc}</div>
          <div style="color: #475569;"><strong>${howLbl}</strong> ${meta.howItWorks}</div>
        </div>
      `;

      if (meta.params && meta.params.length > 0) {
        rhtml += `<div class="param-grid">`;
        meta.params.forEach(p => {
          const curVal = (block.params && block.params[p.key] !== undefined) ? block.params[p.key] : p.default;
          if (p.type === 'select') {
            let opts = '';
            p.options.forEach(opt => {
              opts += `<option value="${opt}" ${curVal === opt ? 'selected' : ''}>${opt}</option>`;
            });
            rhtml += `
              <div class="param-item">
                <div class="param-item-header">
                  <span class="param-name">${p.label}</span>
                </div>
                <select class="win-select rc505-opt-sel" data-block-id="${block.id}" data-param-key="${p.key}">
                  ${opts}
                </select>
              </div>
            `;
          } else {
            rhtml += `
              <div class="param-item">
                <div class="param-item-header">
                  <span class="param-name">${p.label}</span>
                  <span class="param-val-badge" id="badge_${block.id}_${p.key}">${curVal} ${p.unit}</span>
                </div>
                <div class="param-slider-row">
                  <input type="range" class="param-slider rc505-slider" min="${p.min}" max="${p.max}" step="${p.step}" value="${curVal}" data-block-id="${block.id}" data-param-key="${p.key}" data-unit="${p.unit}">
                </div>
              </div>
            `;
          }
        });
        rhtml += `</div>`;
      }

      if (meta.presets && meta.presets.length > 0) {
        rhtml += `
          <div class="preset-pills-row">
            <span class="preset-pill-label">${presetsLbl}</span>
        `;
        meta.presets.forEach((pr, pi) => {
          rhtml += `
            <button class="pill-btn rc505-preset-pill" data-block-id="${block.id}" data-eff-id="${block.effectId}" data-preset-idx="${pi}">
              ${pr.name}
            </button>
          `;
        });
        rhtml += `</div>`;
      }

      content.innerHTML = rhtml;
    }

    card.appendChild(content);
    blocksContainer.appendChild(card);
  });

  attachBlockEvents();

  // Restore scroll position so reordering, deleting or updating never jumps to top/bottom
  if (scrollContainer) {
    if (targetScrollBlockId != null) {
      const targetCard = blocksContainer ? blocksContainer.querySelector(`[data-block-id="${targetScrollBlockId}"]`) : null;
      if (targetCard) {
        targetCard.scrollIntoView({ block: 'nearest', behavior: 'auto' });
      } else {
        scrollContainer.scrollTop = prevScrollTop;
      }
    } else {
      scrollContainer.scrollTop = prevScrollTop;
    }
  }
}

function attachBlockEvents() {
  const tab = getActiveTab();
  if (!tab) return;

  // 1. Toggle Power On/Off
  document.querySelectorAll('.power-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const parentId = btn.dataset.parent;
      if (parentId) {
        const pBlock = tab.blocks.find(x => String(x.id) === String(parentId));
        if (pBlock && pBlock.filters) {
          const flt = pBlock.filters.find(x => String(x.id) === String(btn.dataset.fltId));
          if (flt) {
            flt.enabled = !flt.enabled;
            showToast(flt.enabled ? 'Фильтр включен' : 'Фильтр отключен (Bypass)');
          }
        }
      } else {
        const b = tab.blocks.find(x => String(x.id) === String(btn.dataset.blockId));
        if (b) {
          b.enabled = !b.enabled;
          showToast(b.enabled ? 'Блок включен' : 'Блок отключен (Bypass)');
        }
      }
      updateAll();
      renderBlocks();
    });
  });

  // 2. Delete entire block
  document.querySelectorAll('.filter-block-header .delete').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const id = btn.dataset.blockId;
      tab.blocks = tab.blocks.filter(x => String(x.id) !== String(id));
      updateAll();
      renderBlocks();
      showToast('Блок удален');
    });
  });

  // 3. Move block up/down
  document.querySelectorAll('.move-up').forEach(btn => {
    btn.addEventListener('click', () => {
      const idx = parseInt(btn.dataset.idx);
      if (idx > 0) {
        const movedId = tab.blocks[idx].id;
        const temp = tab.blocks[idx];
        tab.blocks[idx] = tab.blocks[idx - 1];
        tab.blocks[idx - 1] = temp;
        updateAll();
        renderBlocks(movedId);
      }
    });
  });

  document.querySelectorAll('.move-down').forEach(btn => {
    btn.addEventListener('click', () => {
      const idx = parseInt(btn.dataset.idx);
      if (idx < tab.blocks.length - 1) {
        const movedId = tab.blocks[idx].id;
        const temp = tab.blocks[idx];
        tab.blocks[idx] = tab.blocks[idx + 1];
        tab.blocks[idx + 1] = temp;
        updateAll();
        renderBlocks(movedId);
      }
    });
  });

  // 4. Preamp slider & text input
  document.querySelectorAll('.preamp-slider').forEach(slider => {
    const blockId = slider.dataset.blockId;
    const b = tab.blocks.find(x => String(x.id) === String(blockId));
    const input = slider.closest('.preamp-row')?.querySelector('.preamp-input');

    const onUpdate = (e) => {
      const val = parseFloat(e.target.value);
      if (b) {
        b.gain = val;
        if (input) input.value = `${val >= 0 ? '+' : ''}${val.toFixed(2)} dB`;
        updateAll();
      }
    };
    slider.addEventListener('input', onUpdate);
    slider.addEventListener('change', onUpdate);
  });

  document.querySelectorAll('.preamp-input').forEach(inp => {
    const blockId = inp.dataset.blockId;
    const b = tab.blocks.find(x => String(x.id) === String(blockId));
    const slider = inp.closest('.preamp-row')?.querySelector('.preamp-slider');

    const commitInput = () => {
      let num = parseFloat(inp.value.replace(/[^\d.-]/g, ''));
      if (isNaN(num)) num = b ? b.gain : 0.0;
      num = Math.max(-30, Math.min(20, num));
      if (b) {
        b.gain = num;
        inp.value = `${num >= 0 ? '+' : ''}${num.toFixed(2)} dB`;
        if (slider) slider.value = num;
        updateAll();
      }
    };
    inp.addEventListener('change', commitInput);
    inp.addEventListener('blur', commitInput);
    inp.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        commitInput();
        inp.blur();
      }
    });
  });

  // Delay slider & text input
  document.querySelectorAll('.delay-slider').forEach(slider => {
    const blockId = slider.dataset.blockId;
    const b = tab.blocks.find(x => String(x.id) === String(blockId));
    const input = slider.closest('.preamp-row')?.querySelector('.delay-input');

    const onUpdate = (e) => {
      const val = parseFloat(e.target.value);
      if (b) {
        b.delay = Math.max(0, val);
        if (input) input.value = `${b.delay.toFixed(2)} ms`;
        updateAll();
      }
    };
    slider.addEventListener('input', onUpdate);
    slider.addEventListener('change', onUpdate);
  });

  document.querySelectorAll('.delay-input').forEach(inp => {
    const blockId = inp.dataset.blockId;
    const b = tab.blocks.find(x => String(x.id) === String(blockId));
    const slider = inp.closest('.preamp-row')?.querySelector('.delay-slider');

    const commitInput = () => {
      let num = parseFloat(inp.value.replace(/[^\d.-]/g, ''));
      if (isNaN(num)) num = b ? b.delay : 10.0;
      num = Math.max(0, Math.min(2000, num));
      if (b) {
        b.delay = num;
        inp.value = `${num.toFixed(2)} ms`;
        if (slider) slider.value = num;
        updateAll();
      }
    };
    inp.addEventListener('change', commitInput);
    inp.addEventListener('blur', commitInput);
    inp.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        commitInput();
        inp.blur();
      }
    });
  });

  // 5. Parametric table controls (Task 11: dynamic explanation hints & tooltips)
  document.querySelectorAll('.flt-type-sel').forEach(sel => {
    sel.addEventListener('change', (e) => {
      const p = tab.blocks.find(x => String(x.id) === String(sel.dataset.parent));
      if (p && p.filters) {
        const f = p.filters.find(x => String(x.id) === String(sel.dataset.fltId));
        if (f) {
          f.type = e.target.value;
          const hintText = FILTER_HINTS[f.type] || '';
          const td = sel.closest('td');
          const hintBox = td?.querySelector('.flt-hint-text');
          const badge = td?.querySelector('.flt-info-badge');
          if (hintBox) hintBox.textContent = hintText;
          if (badge) badge.title = hintText;
          sel.title = hintText;
          updateAll();
        }
      }
    });
  });

  document.querySelectorAll('.flt-fc').forEach(inp => {
    inp.addEventListener('input', (e) => {
      const p = tab.blocks.find(x => String(x.id) === String(inp.dataset.parent));
      if (p && p.filters) {
        const f = p.filters.find(x => String(x.id) === String(inp.dataset.fltId));
        if (f) {
          f.fc = parseFloat(e.target.value) || 100;
          updateAll();
        }
      }
    });
  });

  document.querySelectorAll('.flt-gain').forEach(inp => {
    inp.addEventListener('input', (e) => {
      const p = tab.blocks.find(x => String(x.id) === String(inp.dataset.parent));
      if (p && p.filters) {
        const f = p.filters.find(x => String(x.id) === String(inp.dataset.fltId));
        if (f) {
          f.gain = parseFloat(e.target.value) || 0;
          updateAll();
        }
      }
    });
  });

  document.querySelectorAll('.flt-q').forEach(inp => {
    inp.addEventListener('input', (e) => {
      const p = tab.blocks.find(x => String(x.id) === String(inp.dataset.parent));
      if (p && p.filters) {
        const f = p.filters.find(x => String(x.id) === String(inp.dataset.fltId));
        if (f) {
          f.q = parseFloat(e.target.value) || 1.41;
          updateAll();
        }
      }
    });
  });

  // 6. Delete individual parametric filter
  document.querySelectorAll('.flt-del-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const p = tab.blocks.find(x => String(x.id) === String(btn.dataset.parent));
      if (p && p.filters) {
        p.filters = p.filters.filter(x => String(x.id) !== String(btn.dataset.fltId));
        updateAll();
        renderBlocks();
        showToast('Полоса фильтра удалена');
      }
    });
  });

  // 7. Add sub-filter
  document.querySelectorAll('.add-sub-flt-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const p = tab.blocks.find(x => String(x.id) === String(btn.dataset.parent));
      if (p && p.filters) {
        p.filters.push({
          id: nextId(),
          type: 'PK',
          fc: 1000,
          gain: 3.0,
          q: 1.4142,
          enabled: true
        });
        updateAll();
        renderBlocks();
        showToast('Добавлена полоса фильтра');
      }
    });
  });

  // 8. Graphic EQ sliders
  document.querySelectorAll('.geq-v-slider').forEach(slider => {
    slider.addEventListener('input', (e) => {
      const bId = slider.dataset.blockId;
      const bIdx = parseInt(slider.dataset.bandIdx);
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (blk && blk.bands && blk.bands[bIdx]) {
        blk.bands[bIdx].gain = parseInt(e.target.value);
        slider.parentElement.querySelector('.geq-db-val').textContent =
          (blk.bands[bIdx].gain > 0 ? '+' : '') + blk.bands[bIdx].gain;
        updateAll();
      }
    });
  });

  // 9. VST Sliders
  document.querySelectorAll('.vst-slider').forEach(slider => {
    slider.addEventListener('input', (e) => {
      const bId = slider.dataset.blockId;
      const key = slider.dataset.paramKey;
      const unit = slider.dataset.unit || '';
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (blk) {
        if (!blk.params) blk.params = {};
        const numVal = parseFloat(e.target.value);
        blk.params[key] = numVal;
        const badge = document.getElementById(`badge_${bId}_${key}`);
        if (badge) badge.textContent = `${numVal} ${unit}`;
        updateAll();
      }
    });
  });

  // 10. VST Library path changes
  document.querySelectorAll('.vst-lib-path').forEach(inp => {
    inp.addEventListener('change', (e) => {
      const bId = inp.dataset.blockId;
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (blk) {
        blk.library = e.target.value.trim();
        updateAll();
      }
    });
  });

  // 11. Boss RC-505 Sliders & Selects
  document.querySelectorAll('.rc505-slider').forEach(slider => {
    slider.addEventListener('input', (e) => {
      const bId = slider.dataset.blockId;
      const key = slider.dataset.paramKey;
      const unit = slider.dataset.unit || '';
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (blk) {
        if (!blk.params) blk.params = {};
        const numVal = parseFloat(e.target.value);
        blk.params[key] = numVal;
        const badge = document.getElementById(`badge_${bId}_${key}`);
        if (badge) badge.textContent = `${numVal} ${unit}`;
        updateAll();
      }
    });
  });

  document.querySelectorAll('.rc505-opt-sel').forEach(sel => {
    sel.addEventListener('change', (e) => {
      const bId = sel.dataset.blockId;
      const key = sel.dataset.paramKey;
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (blk) {
        if (!blk.params) blk.params = {};
        blk.params[key] = e.target.value;
        updateAll();
      }
    });
  });

  // 12. Preset Pill Buttons (VST & RC-505)
  document.querySelectorAll('.vst-preset-pill').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const bId = btn.dataset.blockId;
      const vstId = btn.dataset.vstId;
      const pIdx = parseInt(btn.dataset.presetIdx);
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      const meta = VST_PLUGINS_CATALOG[vstId];
      if (blk && meta && meta.presets && meta.presets[pIdx]) {
        const pr = meta.presets[pIdx];
        blk.params = Object.assign({}, pr.values);
        renderBlocks();
        updateAll();
        showToast(`Пресет VST "${pr.name}" применен`);
      }
    });
  });

  document.querySelectorAll('.rc505-preset-pill').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const bId = btn.dataset.blockId;
      const effId = btn.dataset.effId;
      const pIdx = parseInt(btn.dataset.presetIdx);
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      const meta = RC505_CATALOG[effId];
      if (blk && meta && meta.presets && meta.presets[pIdx]) {
        const pr = meta.presets[pIdx];
        blk.params = Object.assign({}, pr.values);
        renderBlocks();
        updateAll();
        showToast(`Пресет BOSS RC-505 "${pr.name}" применен`);
      }
    });
  });

  // 13. Include Block events (Task: "на скрине не работают кнопки выбрать открыть")
  document.querySelectorAll('.include-path-input').forEach(inp => {
    inp.addEventListener('input', (e) => {
      const bId = inp.dataset.blockId;
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (blk) {
        blk.path = e.target.value.trim();
        updateAll();
      }
    });
  });

  document.querySelectorAll('.btn-include-browse').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const bId = btn.dataset.blockId;
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (!blk) return;

      const fileInp = document.createElement('input');
      fileInp.type = 'file';
      fileInp.accept = '.txt';
      fileInp.onchange = (ev) => {
        const file = ev.target.files && ev.target.files[0];
        if (file) {
          blk.path = file.name;
          renderBlocks();
          updateAll();
          showToast(`Выбран файл конфигурации: ${file.name}`);
        }
      };
      fileInp.click();
    });
  });

  document.querySelectorAll('.btn-include-open').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const bId = btn.dataset.blockId;
      const blk = tab.blocks.find(x => String(x.id) === String(bId));
      if (!blk || !blk.path || !blk.path.trim()) {
        showToast('Укажите имя файла конфигурации', true);
        return;
      }
      const targetName = blk.path.trim();

      // 1. Check if tab already exists
      const existingTab = state.tabs.find(t => t.name.toLowerCase() === targetName.toLowerCase());
      if (existingTab) {
        switchTab(existingTab.id);
        showToast(`Переключено на вкладку: ${targetName}`);
        return;
      }

      // 2. Fetch from server
      let loadedText = '';
      try {
        const res = await fetch(`/api/config?path=${encodeURIComponent(targetName)}`);
        if (res.ok) {
          const data = await res.json();
          loadedText = data.config || '';
        }
      } catch (err) {}

      // 3. Parse or initialize blocks
      let parsedBlocks = [];
      if (loadedText.trim()) {
        parsedBlocks = parseDirectivesToBlocks(loadedText);
      }
      if (parsedBlocks.length === 0) {
        parsedBlocks = [
          { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 }
        ];
      }

      const newTabId = `tab_${nextId()}`;
      state.tabs.push({
        id: newTabId,
        name: targetName,
        selectedDevice: 'all',
        selectedChannel: 'all',
        blocks: parsedBlocks
      });

      switchTab(newTabId);
      saveTabsState();
      showToast(`Файл ${targetName} открыт в новой вкладке`);
    });
  });
}

// --- Sleek Toast Notification Helper ---
function showToast(message, isError = false) {
  const toast = document.getElementById('toastNotification');
  if (!toast) return;
  toast.textContent = message;
  toast.style.borderLeftColor = isError ? '#ef4444' : '#10b981';
  toast.classList.remove('hidden');
  if (window._toastTimer) clearTimeout(window._toastTimer);
  window._toastTimer = setTimeout(() => {
    toast.classList.add('hidden');
  }, 2400);
}

// --- Robust Device Selector Value Helper ---
function setDeviceSelectorValue(sel, targetGuid) {
  if (!sel || !targetGuid) return false;
  const targetLower = targetGuid.toLowerCase().trim();
  for (let i = 0; i < sel.options.length; i++) {
    const optVal = sel.options[i].value.toLowerCase().trim();
    if (optVal === targetLower || (targetLower.length > 8 && optVal.includes(targetLower))) {
      sel.selectedIndex = i;
      const tab = getActiveTab();
      if (tab) tab.selectedDevice = sel.options[i].value;
      localStorage.setItem('resona_selected_device', sel.options[i].value);
      return true;
    }
  }
  return false;
}

// --- Undo / Redo History System ---
let historyDebounceTimer = null;
function recordStateChange() {
  if (isUndoRedoing) return;
  if (historyDebounceTimer) clearTimeout(historyDebounceTimer);
  historyDebounceTimer = setTimeout(() => {
    pushHistory();
  }, 250);
}

function pushHistory() {
  if (isUndoRedoing) return;
  const tab = getActiveTab();
  if (!tab) return;

  const snapshot = JSON.stringify({
    blocks: tab.blocks,
    device: tab.selectedDevice,
    channel: tab.selectedChannel
  });
  if (undoStack.length > 0 && undoStack[undoStack.length - 1] === snapshot) return;
  undoStack.push(snapshot);
  if (undoStack.length > 50) undoStack.shift();
  redoStack = [];
  updateUndoRedoButtons();
}

function undo() {
  if (undoStack.length <= 1) return;
  const current = undoStack.pop();
  redoStack.push(current);
  const prevSnap = JSON.parse(undoStack[undoStack.length - 1]);
  isUndoRedoing = true;
  const tab = getActiveTab();
  if (tab) {
    tab.blocks = prevSnap.blocks;
    tab.selectedDevice = prevSnap.device;
    tab.selectedChannel = prevSnap.channel;
    const devEl = document.getElementById('deviceSelector');
    if (devEl) setDeviceSelectorValue(devEl, tab.selectedDevice);
    const chEl = document.getElementById('channelSelector');
    if (chEl) chEl.value = tab.selectedChannel;
  }
  renderBlocks();
  renderAnalysis();
  rawConfigText.value = generateConfig();
  triggerSync();
  isUndoRedoing = false;
  updateUndoRedoButtons();
  showToast("Отмена действия");
}

function redo() {
  if (redoStack.length === 0) return;
  const nextSnapStr = redoStack.pop();
  undoStack.push(nextSnapStr);
  const nextSnap = JSON.parse(nextSnapStr);
  isUndoRedoing = true;
  const tab = getActiveTab();
  if (tab) {
    tab.blocks = nextSnap.blocks;
    tab.selectedDevice = nextSnap.device;
    tab.selectedChannel = nextSnap.channel;
    const devEl = document.getElementById('deviceSelector');
    if (devEl) setDeviceSelectorValue(devEl, tab.selectedDevice);
    const chEl = document.getElementById('channelSelector');
    if (chEl) chEl.value = tab.selectedChannel;
  }
  renderBlocks();
  renderAnalysis();
  rawConfigText.value = generateConfig();
  triggerSync();
  isUndoRedoing = false;
  updateUndoRedoButtons();
  showToast("Повтор действия");
}

function updateUndoRedoButtons() {
  const canUndo = undoStack.length > 1;
  const canRedo = redoStack.length > 0;

  const tbUndo = document.getElementById('tbUndo');
  const tbRedo = document.getElementById('tbRedo');
  if (tbUndo) {
    tbUndo.style.opacity = canUndo ? '1' : '0.35';
    tbUndo.style.cursor = canUndo ? 'pointer' : 'default';
  }
  if (tbRedo) {
    tbRedo.style.opacity = canRedo ? '1' : '0.35';
    tbRedo.style.cursor = canRedo ? 'pointer' : 'default';
  }

  const menuUndo = document.getElementById('menuUndo');
  const menuRedo = document.getElementById('menuRedo');
  if (menuUndo) {
    menuUndo.style.opacity = canUndo ? '1' : '0.4';
    menuUndo.disabled = !canUndo;
  }
  if (menuRedo) {
    menuRedo.style.opacity = canRedo ? '1' : '0.4';
    menuRedo.disabled = !canRedo;
  }
}

// --- Parse Directives to Blocks (Helper) ---
function parseDirectivesToBlocks(text) {
  const dummyTab = { blocks: [], selectedDevice: 'all', selectedChannel: 'all' };
  parseConfigIntoState(text, dummyTab);
  return dummyTab.blocks;
}

// --- Parse config.txt Text into active tab ---
function parseConfigIntoState(text, targetTab = null) {
  if (!text || typeof text !== 'string') return false;
  const lines = text.split(/\r?\n/);
  const newBlocks = [];
  let currentParametric = null;
  let pendingVstMeta = null;
  let pendingRc505Meta = null;
  const tab = targetTab || getActiveTab();
  if (tab) {
    tab.selectedDevice = 'all';
    tab.selectedChannel = 'all';
  }

  for (let rawLine of lines) {
    let line = rawLine.trim();
    if (!line) continue;

    let isExplicitDisabled = false;
    if (line.startsWith('# [Disabled]')) {
      isExplicitDisabled = true;
      line = line.replace(/^#\s*\[Disabled\]\s*/i, '').trim();
    }

    const vstMetaMatch = line.match(/^#?\s*\[VSTPlugin:\s*([^\]]+)\]\s*Params:\s*(\{.*\})/i);
    if (vstMetaMatch) {
      try {
        pendingVstMeta = { id: vstMetaMatch[1].trim(), params: JSON.parse(vstMetaMatch[2]) };
      } catch (e) {}
      continue;
    }

    const rc505MetaMatch = line.match(/^#?\s*\[BossRC505:\s*([^\]]+)\]\s*Params:\s*(\{.*\})/i);
    if (rc505MetaMatch) {
      try {
        pendingRc505Meta = { id: rc505MetaMatch[1].trim(), params: JSON.parse(rc505MetaMatch[2]) };
      } catch (e) {}
      continue;
    }

    if (line.startsWith('#')) {
      continue;
    }

    if (/^Device:\s*all\b/i.test(line)) {
      if (tab) tab.selectedDevice = 'all';
      const sel = document.getElementById('deviceSelector');
      if (sel) sel.value = 'all';
      continue;
    }

    const devMatch = line.match(/^Device:\s*(?:"([^"]+)"\s*)?(\{[-a-fA-F0-9]+\})/i) || line.match(/^Device:\s*([^{}\r\n]+)\s*(\{[-a-fA-F0-9]+\})/i);
    if (devMatch) {
      const guid = devMatch[2] || devMatch[1];
      if (tab) tab.selectedDevice = guid;
      const sel = document.getElementById('deviceSelector');
      if (sel) setDeviceSelectorValue(sel, guid);
      continue;
    }

    const chMatch = line.match(/^Channel:\s*(.+)$/i);
    if (chMatch) {
      const ch = chMatch[1].trim();
      if (tab) tab.selectedChannel = ch;
      const sel = document.getElementById('channelSelector');
      if (sel) sel.value = ch;
      continue;
    }

    const preampMatch = line.match(/^Preamp:\s*([+-]?\d+(?:\.\d+)?)\s*dB/i);
    if (preampMatch) {
      currentParametric = null;
      newBlocks.push({
        id: nextId(),
        kind: 'preamp',
        title: 'Предусиление (Preamp)',
        enabled: !isExplicitDisabled,
        gain: parseFloat(preampMatch[1])
      });
      continue;
    }

    const incMatch = line.match(/^Include:\s*(.+)$/i);
    if (incMatch) {
      currentParametric = null;
      newBlocks.push({
        id: nextId(),
        kind: 'include',
        title: 'Include file',
        enabled: !isExplicitDisabled,
        path: incMatch[1].trim()
      });
      continue;
    }

    const vstMatch = line.match(/^VSTPlugin:\s*Library\s+"?([^"\r\n]+)"?/i);
    if (vstMatch) {
      currentParametric = null;
      const libPath = vstMatch[1].trim();
      let vstKey = pendingVstMeta ? pendingVstMeta.id : null;
      if (!vstKey || !VST_PLUGINS_CATALOG[vstKey]) {
        const lower = libPath.toLowerCase();
        for (const k of Object.keys(VST_PLUGINS_CATALOG)) {
          if (lower.includes(VST_PLUGINS_CATALOG[k].dll.toLowerCase())) {
            vstKey = k;
            break;
          }
        }
        if (!vstKey && Array.isArray(ALL_VST_98_CATALOG)) {
          const match98 = ALL_VST_98_CATALOG.find(p => lower.includes(p.dll.toLowerCase()));
          if (match98) {
            vstKey = match98.id;
          }
        }
      }
      const vstBlock = createVstBlock(vstKey || 'custom', libPath);
      vstBlock.enabled = !isExplicitDisabled;
      if (pendingVstMeta && pendingVstMeta.params) {
        vstBlock.params = Object.assign(vstBlock.params, pendingVstMeta.params);
      }
      newBlocks.push(vstBlock);
      pendingVstMeta = null;
      continue;
    }

    const rc505Match = line.match(/^BossRC505:\s*Effect\s+"?([^"\r\n]+)"?/i);
    if (rc505Match) {
      currentParametric = null;
      let effKey = pendingRc505Meta ? pendingRc505Meta.id : null;
      if (!effKey || !RC505_CATALOG[effKey]) {
        const titleMatch = rc505Match[1].toLowerCase();
        for (const k of Object.keys(RC505_CATALOG)) {
          if (titleMatch.includes(k) || RC505_CATALOG[k].title.toLowerCase().includes(titleMatch)) {
            effKey = k;
            break;
          }
        }
      }
      const rcBlock = createRc505Block(effKey || 'slicer');
      rcBlock.enabled = !isExplicitDisabled;
      if (pendingRc505Meta && pendingRc505Meta.params) {
        rcBlock.params = Object.assign(rcBlock.params, pendingRc505Meta.params);
      }
      newBlocks.push(rcBlock);
      pendingRc505Meta = null;
      continue;
    }

    const fltMatch = line.match(/^Filter(?:\s*\d+)?:\s*(ON|OFF)?\s*([A-Za-z]+)\s+Fc\s+(\d+(?:\.\d+)?)\s*Hz\s+Gain\s*([+-]?\d+(?:\.\d+)?)\s*dB(?:\s+Q\s+(\d+(?:\.\d+)?))?/i);
    if (fltMatch) {
      const filterOn = fltMatch[1] ? fltMatch[1].toUpperCase() !== 'OFF' : true;
      let fType = fltMatch[2].toUpperCase();
      if (fType === 'PEAKING') fType = 'PK';
      if (fType === 'LOWSHELF') fType = 'LS';
      if (fType === 'HIGHSHELF') fType = 'HS';
      if (fType === 'LOWPASS') fType = 'LP';
      if (fType === 'HIGHPASS') fType = 'HP';
      if (fType === 'NOTCH') fType = 'NO';

      const fc = parseFloat(fltMatch[3]);
      const gain = parseFloat(fltMatch[4]);
      const q = fltMatch[5] ? parseFloat(fltMatch[5]) : 1.4142;

      if (!currentParametric) {
        currentParametric = {
          id: nextId(),
          kind: 'parametric',
          title: 'Параметрические фильтры (Biquad EQ)',
          enabled: !isExplicitDisabled,
          filters: []
        };
        newBlocks.push(currentParametric);
      }

      currentParametric.filters.push({
        id: nextId(),
        type: fType,
        fc: fc,
        gain: gain,
        q: q,
        enabled: isExplicitDisabled ? false : filterOn
      });
      continue;
    }

    const geqMatch = line.match(/^GraphicEQ:\s*(.+)$/i);
    if (geqMatch) {
      currentParametric = null;
      const pts = geqMatch[1].split(';');
      const parsedPoints = [];

      for (let p of pts) {
        const parts = p.trim().split(/\s+/);
        if (parts.length >= 2) {
          const f = parseFloat(parts[0]);
          const g = parseFloat(parts[1]);
          if (!isNaN(f) && !isNaN(g)) {
            parsedPoints.push({ f, g });
          }
        }
      }

      const is31 = parsedPoints.length > 20;
      const ISO_FREQS = is31
        ? [20, 25, 31.5, 40, 50, 63, 80, 100, 125, 160, 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000, 10000, 12500, 16000, 20000]
        : [25, 40, 63, 100, 160, 250, 400, 630, 1000, 1600, 2500, 4000, 6300, 10000, 16000];

      const bands = ISO_FREQS.map(isoF => {
        let nearestG = 0;
        let minDiff = Infinity;
        for (let pt of parsedPoints) {
          const diff = Math.abs(Math.log10(pt.f) - Math.log10(isoF));
          if (diff < minDiff) {
            minDiff = diff;
            nearestG = pt.g;
          }
        }
        return { f: isoF, gain: Math.round(nearestG) };
      });

      newBlocks.push({
        id: nextId(),
        kind: is31 ? 'graphiceq31' : 'graphiceq15',
        title: is31 ? 'Графический эквалайзер (31 полоса ISO)' : 'Графический эквалайзер (15 полос ISO)',
        enabled: !isExplicitDisabled,
        bands: bands
      });
      continue;
    }

    const delayMatch = line.match(/^Delay:\s*([\d.]+)\s*(?:ms)?/i);
    if (delayMatch) {
      currentParametric = null;
      newBlocks.push({
        id: nextId(),
        kind: 'delay',
        title: 'Задержка канала (Delay)',
        enabled: !isExplicitDisabled,
        delay: parseFloat(delayMatch[1]) || 0.0
      });
      continue;
    }
  }

  if (newBlocks.length > 0 && tab) {
    tab.blocks = newBlocks;
    return true;
  }
  return false;
}

// --- Generate config.txt from Active Tab ---
function generateConfig() {
  const tab = getActiveTab();
  if (!tab) return '';

  let text = `# =========================================================================\n`;
  text += `# Equalizer FD CRAFT Configuration file: ${tab.name}\n`;
  text += `# Generated by Equalizer FD CRAFT Studio Engine\n`;
  text += `# =========================================================================\n\n`;

  if (tab.selectedDevice && tab.selectedDevice !== 'all') {
    const sel = document.getElementById('deviceSelector');
    const selectedOpt = sel ? sel.options[sel.selectedIndex] : null;
    let devLabel = selectedOpt ? selectedOpt.textContent : '';
    devLabel = devLabel.replace(/^\[(?:Capture|Render)\]\s*/i, '').trim();
    if (devLabel) {
      text += `Device: "${devLabel}" ${tab.selectedDevice}\n\n`;
    } else {
      text += `Device: ${tab.selectedDevice}\n\n`;
    }
  }

  const chVal = tab.selectedChannel;
  if (chVal && chVal !== 'all') {
    text += `Channel: ${chVal}\n\n`;
  }

  tab.blocks.forEach(b => {
    const prefix = b.enabled ? '' : '# [Disabled] ';
    if (b.kind === 'preamp') {
      text += `${prefix}Preamp: ${b.gain >= 0 ? '+' : ''}${b.gain.toFixed(2)} dB\n`;
    } else if (b.kind === 'delay') {
      text += `${prefix}Delay: ${(b.delay || 0.0).toFixed(2)} ms\n`;
    } else if (b.kind === 'include') {
      text += `${prefix}Include: ${b.path}\n`;
    } else if (b.kind === 'vst') {
      text += `# [VSTPlugin: ${b.vstId || 'custom'}] Params: ${JSON.stringify(b.params || {})}\n`;
      text += `${prefix}VSTPlugin: Library "${b.library || (VST_BASE_DIR + (VST_PLUGINS_CATALOG[b.vstId]?.dll || 'plugin.dll'))}"\n\n`;
    } else if (b.kind === 'rc505') {
      text += `# [BossRC505: ${b.effectId}] Params: ${JSON.stringify(b.params || {})}\n`;
      text += `${prefix}BossRC505: Effect "${RC505_CATALOG[b.effectId]?.title || b.title}" Params "${encodeURIComponent(JSON.stringify(b.params || {}))}"\n\n`;
    } else if (b.kind === 'parametric') {
      b.filters.forEach(f => {
        text += `${prefix}Filter: ${f.enabled ? 'ON' : 'OFF'} ${f.type} Fc ${Math.round(f.fc)} Hz Gain ${f.gain >= 0 ? '+' : ''}${f.gain.toFixed(2)} dB Q ${f.q.toFixed(4)}\n`;
      });
    } else if (b.kind === 'graphiceq15' || b.kind === 'graphiceq31') {
      text += `${prefix}GraphicEQ: `;
      b.bands.forEach((band, bi) => {
        text += `${band.f} ${band.gain}${bi < b.bands.length - 1 ? '; ' : '\n'}`;
      });
    }
  });
  return text;
}

let syncDebounceTimer = null;
function triggerSync() {
  const instantMode = document.getElementById('instantModeCheck');
  if (instantMode && instantMode.checked) {
    if (syncDebounceTimer) clearTimeout(syncDebounceTimer);
    syncDebounceTimer = setTimeout(() => {
      saveConfigToDisk(true);
    }, 200);
  }
}

async function saveConfigToDisk(silent = false) {
  const configText = generateConfig();
  saveTabsState();
  localStorage.setItem('resona_instant_mode', document.getElementById('instantModeCheck')?.checked ? 'true' : 'false');

  try {
    const tab = getActiveTab();
    const filename = tab ? tab.name : 'config.txt';
    const res = await fetch('/api/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ filename, config: configText })
    });
    const data = await res.json();
    if (data.status === 'saved' || data.status === 'saved_local') {
      if (!silent) {
        showToast(filename === 'config.txt' ? 'Конфигурация сохранена в Equalizer FD CRAFT' : `Файл ${filename} сохранен`);
      }
    }
  } catch (err) {
    if (!silent) {
      showToast('Сохранено в локальной памяти браузера', true);
    }
  }
}

function updateAll(recordHistory = true) {
  renderAnalysis();
  rawConfigText.value = generateConfig();
  triggerSync();
  saveTabsState();
  if (recordHistory) {
    recordStateChange();
  }
}

// --- Green Plus Button Menu Handlers ---
addBlockBtn.addEventListener('click', (e) => {
  e.stopPropagation();
  const rect = addBlockBtn.getBoundingClientRect();
  const spaceBelow = window.innerHeight - rect.bottom - 210; // 210px analysis panel
  if (spaceBelow < 260) {
    addMenuDropdown.classList.remove('open-down');
  } else {
    addMenuDropdown.classList.add('open-down');
  }
  addMenuDropdown.classList.toggle('hidden');
});

document.addEventListener('click', () => {
  addMenuDropdown.classList.add('hidden');
});

document.querySelectorAll('.sub-menu button').forEach(btn => {
  btn.addEventListener('click', (e) => {
    const targetBtn = e.target.closest('button');
    const action = targetBtn ? targetBtn.dataset.action : e.target.dataset.action;
    addMenuDropdown.classList.add('hidden');
    const tab = getActiveTab();
    if (!tab) return;

    if (action === 'open-vst-catalog') {
      openVstCatalogModal();
      return;
    }

    if (action === 'add-preamp') {
      tab.blocks.push({ id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 });
    } else if (action === 'add-include') {
      tab.blocks.push({ id: nextId(), kind: 'include', title: 'Include file', enabled: true, path: 'example.txt' });
    } else if (action === 'add-device') {
      openConfigurator();
      return;
    } else if (action === 'add-channel') {
      document.getElementById('channelSelector')?.focus();
      return;
    } else if (action === 'add-peaking') {
      addParametricWithFilter('PK', 1000, 3.0, 1.4142);
    } else if (action === 'add-lowshelf') {
      addParametricWithFilter('LS', 100, 4.0, 0.7071);
    } else if (action === 'add-highshelf') {
      addParametricWithFilter('HS', 8000, 3.0, 0.7071);
    } else if (action === 'add-lowpass') {
      addParametricWithFilter('LP', 5000, 0.0, 0.7071);
    } else if (action === 'add-highpass') {
      addParametricWithFilter('HP', 80, 0.0, 0.7071);
    } else if (action === 'add-notch') {
      addParametricWithFilter('NO', 50, 0.0, 10.0);
    } else if (action === 'add-geq15') {
      tab.blocks.push({
        id: nextId(),
        kind: 'graphiceq15',
        title: 'Графический эквалайзер (15 полос ISO)',
        enabled: true,
        bands: [
          { f: 25, gain: 0 }, { f: 40, gain: 0 }, { f: 63, gain: 0 },
          { f: 100, gain: 0 }, { f: 160, gain: 0 }, { f: 250, gain: 0 },
          { f: 400, gain: 0 }, { f: 630, gain: 0 }, { f: 1000, gain: 0 },
          { f: 1600, gain: 0 }, { f: 2500, gain: 0 }, { f: 4000, gain: 0 },
          { f: 6300, gain: 0 }, { f: 10000, gain: 0 }, { f: 16000, gain: 0 }
        ]
      });
      showToast('Добавлен: Графический эквалайзер 15 полос');
    } else if (action === 'add-geq31') {
      tab.blocks.push({
        id: nextId(),
        kind: 'graphiceq31',
        title: 'Графический эквалайзер (31 полоса ISO)',
        enabled: true,
        bands: [
          { f: 20, gain: 0 }, { f: 25, gain: 0 }, { f: 31.5, gain: 0 },
          { f: 40, gain: 0 }, { f: 50, gain: 0 }, { f: 63, gain: 0 },
          { f: 80, gain: 0 }, { f: 100, gain: 0 }, { f: 125, gain: 0 },
          { f: 160, gain: 0 }, { f: 200, gain: 0 }, { f: 250, gain: 0 },
          { f: 315, gain: 0 }, { f: 400, gain: 0 }, { f: 500, gain: 0 },
          { f: 630, gain: 0 }, { f: 800, gain: 0 }, { f: 1000, gain: 0 },
          { f: 1250, gain: 0 }, { f: 1600, gain: 0 }, { f: 2000, gain: 0 },
          { f: 2500, gain: 0 }, { f: 3150, gain: 0 }, { f: 4000, gain: 0 },
          { f: 5000, gain: 0 }, { f: 6300, gain: 0 }, { f: 8000, gain: 0 },
          { f: 10000, gain: 0 }, { f: 12500, gain: 0 }, { f: 16000, gain: 0 },
          { f: 20000, gain: 0 }
        ]
      });
      showToast('Добавлен: Графический эквалайзер 31 полоса');
    } else if (action === 'add-vst-reagate') {
      tab.blocks.push(createVstBlock('reagate'));
      showToast('Добавлен VST: ReaGate');
    } else if (action === 'add-vst-reacomp') {
      tab.blocks.push(createVstBlock('reacomp'));
      showToast('Добавлен VST: ReaComp');
    } else if (action === 'add-vst-reafir') {
      tab.blocks.push(createVstBlock('reafir'));
      showToast('Добавлен VST: ReaFIR');
    } else if (action === 'add-vst-readelay') {
      tab.blocks.push(createVstBlock('readelay'));
      showToast('Добавлен VST: ReaDelay');
    } else if (action === 'add-vst-reaxcomp') {
      tab.blocks.push(createVstBlock('reaxcomp'));
      showToast('Добавлен VST: ReaXcomp');
    } else if (action === 'add-vst-basiq') {
      tab.blocks.push(createVstBlock('basiq'));
      showToast('Добавлен VST: basiQ EQ');
    } else if (action === 'add-vst-custom') {
      openCustomVstModal();
      return;
    } else if (action === 'add-rc505-slicer') {
      tab.blocks.push(createRc505Block('slicer'));
      showToast('Добавлен BOSS RC-505: Slicer');
    } else if (action === 'add-rc505-vinyl') {
      tab.blocks.push(createRc505Block('vinyl'));
      showToast('Добавлен BOSS RC-505: Vinyl Flick');
    } else if (action === 'add-rc505-vocoder') {
      tab.blocks.push(createRc505Block('vocoder'));
      showToast('Добавлен BOSS RC-505: Vocoder');
    } else if (action === 'add-rc505-ringmod') {
      tab.blocks.push(createRc505Block('ringmod'));
      showToast('Добавлен BOSS RC-505: Ring Modulator');
    } else if (action === 'add-rc505-isolator') {
      tab.blocks.push(createRc505Block('isolator'));
      showToast('Добавлен BOSS RC-505: Isolator');
    } else if (action === 'add-rc505-lofi') {
      tab.blocks.push(createRc505Block('lofi'));
      showToast('Добавлен BOSS RC-505: Lo-Fi Radio');
    } else if (action === 'add-rc505-chorus') {
      tab.blocks.push(createRc505Block('chorus'));
      showToast('Добавлен BOSS RC-505: Space Chorus');
    } else if (action === 'add-rc505-distortion') {
      tab.blocks.push(createRc505Block('distortion'));
      showToast('Добавлен BOSS RC-505: Distortion');
    } else if (action === 'add-rc505-scatter') {
      tab.blocks.push(createRc505Block('scatter'));
      showToast('Добавлен BOSS RC-505: Granular Scatter');
    } else if (action === 'add-rc505-reverb') {
      tab.blocks.push(createRc505Block('reverb'));
      showToast('Добавлен BOSS RC-505: Space Reverb');
    } else if (action === 'add-delay') {
      tab.blocks.push({
        id: nextId(),
        kind: 'delay',
        title: 'Задержка канала (Delay)',
        enabled: true,
        delay: 10.0
      });
      showToast('Добавлен фильтр: Задержка (Delay)');
    }

    renderBlocks();
    updateAll();
  });
});

function addParametricWithFilter(type, fc, gain, q) {
  const tab = getActiveTab();
  if (!tab) return;

  let paramBlock = tab.blocks.find(x => x.kind === 'parametric');
  if (!paramBlock) {
    paramBlock = {
      id: nextId(),
      kind: 'parametric',
      title: 'Параметрические фильтры (Biquad EQ)',
      enabled: true,
      filters: []
    };
    tab.blocks.push(paramBlock);
  }
  paramBlock.filters.push({
    id: nextId(),
    type, fc, gain, q, enabled: true
  });
}

// Draggable nodes on canvas
canvas.addEventListener('mousedown', (e) => {
  const rect = canvas.getBoundingClientRect();
  const x = (e.clientX - rect.left) * (canvas.width / rect.width);
  const y = (e.clientY - rect.top) * (canvas.height / rect.height);
  const tab = getActiveTab();
  if (!tab) return;

  tab.blocks.forEach(b => {
    if (b.kind === 'parametric' && b.enabled) {
      b.filters.forEach(flt => {
        if (!flt.enabled) return;
        const nx = fToX(flt.fc, canvas.width);
        const ny = dbToY(flt.gain, canvas.height);
        if (Math.hypot(x - nx, y - ny) < 14) {
          state.draggingNode = flt;
        }
      });
    }
  });
});

window.addEventListener('mousemove', (e) => {
  const rect = canvas.getBoundingClientRect();
  const x = (e.clientX - rect.left) * (canvas.width / rect.width);
  const y = (e.clientY - rect.top) * (canvas.height / rect.height);

  if (state.draggingNode) {
    state.draggingNode.fc = Math.max(20, Math.min(20000, Math.round(xToF(x, canvas.width))));
    state.draggingNode.gain = Math.max(-25, Math.min(25, Math.round(yToDb(y, canvas.height) * 2) / 2));
    updateAll();
  }

  // Hover Tooltip with Musical Note name
  if (x >= 0 && x <= canvas.width && y >= 0 && y <= canvas.height) {
    const f = Math.round(xToF(x, canvas.width));
    const db = yToDb(y, canvas.height).toFixed(1);
    const note = freqToNoteName(f);
    curveTooltip.style.left = `${(x / canvas.width) * 100}%`;
    curveTooltip.style.top = `${(y / canvas.height) * 100}%`;
    curveTooltip.textContent = `${f} Hz ${note ? '(' + note + ')' : ''} | ${db > 0 ? '+' : ''}${db} dB`;
    curveTooltip.classList.remove('hidden');
  } else {
    curveTooltip.classList.add('hidden');
  }
});

function freqToNoteName(freq) {
  if (freq < 16 || freq > 20000) return '';
  const notes = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
  const midiNum = Math.round(69 + 12 * Math.log2(freq / 440));
  const note = notes[((midiNum % 12) + 12) % 12];
  const octave = Math.floor(midiNum / 12) - 1;
  return `${note}${octave}`;
}

window.addEventListener('mouseup', () => {
  if (state.draggingNode) {
    state.draggingNode = null;
    pushHistory();
    renderBlocks();
  }
});

// Double click on canvas to add a new parametric filter node
canvas.addEventListener('dblclick', (e) => {
  const rect = canvas.getBoundingClientRect();
  const x = (e.clientX - rect.left) * (canvas.width / rect.width);
  const y = (e.clientY - rect.top) * (canvas.height / rect.height);
  const f = Math.max(20, Math.min(20000, Math.round(xToF(x, canvas.width))));
  const gain = Math.max(-20, Math.min(20, Math.round(yToDb(y, canvas.height) * 2) / 2));
  addParametricWithFilter('PK', f, gain, 1.4142);
  renderBlocks();
  updateAll();
  showToast(`Добавлен фильтр: ${f} Hz, ${gain > 0 ? '+' : ''}${gain} dB`);
});

// Mouse wheel on canvas node to dynamically adjust Q (bandwidth)
canvas.addEventListener('wheel', (e) => {
  const rect = canvas.getBoundingClientRect();
  const x = (e.clientX - rect.left) * (canvas.width / rect.width);
  const y = (e.clientY - rect.top) * (canvas.height / rect.height);
  const tab = getActiveTab();
  if (!tab) return;

  let targetFlt = null;
  tab.blocks.forEach(b => {
    if (b.kind === 'parametric' && b.enabled) {
      b.filters.forEach(flt => {
        if (!flt.enabled) return;
        const nx = fToX(flt.fc, canvas.width);
        const ny = dbToY(flt.gain, canvas.height);
        if (Math.hypot(x - nx, y - ny) < 18) {
          targetFlt = flt;
        }
      });
    }
  });

  if (targetFlt) {
    e.preventDefault();
    const delta = e.deltaY < 0 ? 0.15 : -0.15;
    targetFlt.q = Math.max(0.1, Math.min(20, Math.round((targetFlt.q + delta) * 100) / 100));
    updateAll();
    renderBlocks();
    showToast(`Добротность Q: ${targetFlt.q.toFixed(2)}`);
  }
}, { passive: false });

// --- Floating & Draggable Analysis Panel Controller ---
const floatPanelBtn = document.getElementById('floatPanelBtn');
const panelResizeHandle = document.getElementById('panelResizeHandle');
const analysisPanelHeader = document.getElementById('analysisPanelHeader');

function updateFloatBtnText(isFloating) {
  if (!floatPanelBtn) return;
  const key = isFloating ? 'analysis.floatBtnDock' : 'analysis.floatBtnUndock';
  const defaultText = isFloating ? 'Прикрепить' : 'Открепить';
  floatPanelBtn.setAttribute('data-i18n', key);
  floatPanelBtn.textContent = (window.i18n && typeof window.i18n.t === 'function')
    ? window.i18n.t(key, null, defaultText)
    : defaultText;
}

function resizeCanvasToContainer() {
  resizeAnalysisCanvas();
}

if (floatPanelBtn) {
  floatPanelBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    const isFloating = analysisPanel.classList.toggle('floating');
    if (isFloating) {
      analysisPanel.classList.remove('collapsed');
      analysisPanel.style.left = 'calc(50% - 380px)';
      analysisPanel.style.top = '160px';
      analysisPanel.style.width = '760px';
      analysisPanel.style.height = '320px';
      updateFloatBtnText(true);
      showToast('Панель АЧХ откреплена (плавающий режим)');
    } else {
      analysisPanel.style.left = '';
      analysisPanel.style.top = '';
      analysisPanel.style.width = '';
      analysisPanel.style.height = '';
      updateFloatBtnText(false);
      showToast('Панель АЧХ прикреплена к нижней границе');
    }
    setTimeout(resizeCanvasToContainer, 30);
  });
}

// Dragging floating panel by its header
if (analysisPanelHeader) {
  let isDraggingPanel = false;
  let dragStartX = 0;
  let dragStartY = 0;
  let panelStartLeft = 0;
  let panelStartTop = 0;

  analysisPanelHeader.addEventListener('mousedown', (e) => {
    if (!analysisPanel.classList.contains('floating')) return;
    if (e.target.tagName === 'BUTTON' || e.target.closest('button')) return;

    isDraggingPanel = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    const rect = analysisPanel.getBoundingClientRect();
    panelStartLeft = rect.left;
    panelStartTop = rect.top;
    e.preventDefault();
  });

  window.addEventListener('mousemove', (e) => {
    if (!isDraggingPanel) return;
    const dx = e.clientX - dragStartX;
    const dy = e.clientY - dragStartY;
    analysisPanel.style.left = `${Math.max(10, Math.min(window.innerWidth - 100, panelStartLeft + dx))}px`;
    analysisPanel.style.top = `${Math.max(10, Math.min(window.innerHeight - 50, panelStartTop + dy))}px`;
  });

  window.addEventListener('mouseup', () => {
    isDraggingPanel = false;
  });
}

// Resizing floating panel via corner handle
if (panelResizeHandle) {
  let isResizing = false;
  let resizeStartX = 0;
  let resizeStartY = 0;
  let panelStartW = 0;
  let panelStartH = 0;
  let resizeRaf = null;

  panelResizeHandle.addEventListener('mousedown', (e) => {
    isResizing = true;
    resizeStartX = e.clientX;
    resizeStartY = e.clientY;
    panelStartW = analysisPanel.offsetWidth;
    panelStartH = analysisPanel.offsetHeight;
    e.preventDefault();
    e.stopPropagation();
  });

  window.addEventListener('mousemove', (e) => {
    if (!isResizing) return;
    const dw = e.clientX - resizeStartX;
    const dh = e.clientY - resizeStartY;
    const newW = Math.max(440, Math.min(window.innerWidth - 40, panelStartW + dw));
    const newH = Math.max(180, Math.min(window.innerHeight - 60, panelStartH + dh));
    analysisPanel.style.width = `${newW}px`;
    analysisPanel.style.height = `${newH}px`;
    if (!resizeRaf) {
      resizeRaf = requestAnimationFrame(() => {
        resizeCanvasToContainer();
        resizeRaf = null;
      });
    }
  });

  window.addEventListener('mouseup', () => {
    if (isResizing) {
      isResizing = false;
      if (resizeRaf) {
        cancelAnimationFrame(resizeRaf);
        resizeRaf = null;
      }
      resizeCanvasToContainer();
    }
  });
}

// --- Minimize to Tray on Close (Settings) ---
let minimizeToTrayOnClose = localStorage.getItem('eq_craft_minimize_to_tray') !== 'false'; // default true

function updateMinimizeToTrayUI() {
  const chk = document.getElementById('chkMinimizeToTray');
  if (chk) {
    chk.textContent = minimizeToTrayOnClose ? '[Вкл]' : '[Выкл]';
  }
}
updateMinimizeToTrayUI();

document.getElementById('menuMinimizeToTray')?.addEventListener('click', () => {
  minimizeToTrayOnClose = !minimizeToTrayOnClose;
  localStorage.setItem('eq_craft_minimize_to_tray', minimizeToTrayOnClose ? 'true' : 'false');
  updateMinimizeToTrayUI();
  showToast(minimizeToTrayOnClose
    ? 'Сворачивание в трей при закрытии включено'
    : 'Сворачивание в трей отключено (закрытие [X] завершит программу)'
  );
});

// Window Controls (Frameless Titlebar)
const btnWinMin = document.getElementById('btnWinMin');
const btnWinMax = document.getElementById('btnWinMax');
const btnWinClose = document.getElementById('btnWinClose');

if (btnWinMin) {
  btnWinMin.addEventListener('click', () => {
    fetch('/api/window/minimize', { method: 'POST' });
  });
}

if (btnWinMax) {
  btnWinMax.addEventListener('click', () => {
    fetch('/api/window/maximize', { method: 'POST' });
  });
}

async function handleWindowClose() {
  if (minimizeToTrayOnClose) {
    try {
      await fetch('/api/window/hide', { method: 'POST' });
      showToast('Приложение свернуто в системный трей');
    } catch (e) {
      exitApplication();
    }
  } else {
    exitApplication();
  }
}

if (btnWinClose) {
  btnWinClose.addEventListener('click', () => {
    handleWindowClose();
  });
}

window.addEventListener('keydown', (e) => {
  if (e.altKey && (e.key === 'F4' || e.keyCode === 115)) {
    e.preventDefault();
    handleWindowClose();
  }
});

window.addEventListener('pagehide', () => {
  if (!minimizeToTrayOnClose) {
    try {
      navigator.sendBeacon('/api/exit');
    } catch (e) {}
  }
});

// Window Controls Dragging & Native Window Actions
const menubarEl = document.querySelector('.win-menubar');
if (menubarEl) {
  let isDraggingWindow = false;
  let dragStartX = 0;
  let dragStartY = 0;

  menubarEl.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if (e.target.closest('.menu-item') || e.target.closest('.window-controls') || e.target.closest('button')) {
      return;
    }
    isDraggingWindow = true;
    dragStartX = e.screenX;
    dragStartY = e.screenY;
  });

  window.addEventListener('mousemove', (e) => {
    if (!isDraggingWindow) return;
    const dx = e.screenX - dragStartX;
    const dy = e.screenY - dragStartY;
    if (dx !== 0 || dy !== 0) {
      dragStartX = e.screenX;
      dragStartY = e.screenY;
      fetch(`/api/window/move?dx=${dx}&dy=${dy}`, { method: 'POST' }).catch(() => {});
    }
  });

  window.addEventListener('mouseup', () => {
    isDraggingWindow = false;
  });

  menubarEl.addEventListener('dblclick', (e) => {
    if (e.target.closest('.menu-item') || e.target.closest('.window-controls') || e.target.closest('button')) {
      return;
    }
    fetch('/api/window/maximize', { method: 'POST' }).catch(() => {});
  });
}

// Prevent browser F11 fullscreen override
window.addEventListener('keydown', (e) => {
  if (e.key === 'F11' || e.keyCode === 122) {
    e.preventDefault();
  }
});

// --- Device Configurator ---
let cachedEndpoints = [];
let currentCfgTab = 'capture';

document.querySelectorAll('.cfg-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.cfg-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    currentCfgTab = tab.dataset.tab;
    renderConfiguratorDevices();
  });
});

if (cfgTroubleCheck) {
  cfgTroubleCheck.addEventListener('change', () => {
    cfgTroubleBox.classList.toggle('hidden', !cfgTroubleCheck.checked);
  });
}

document.getElementById('closeConfiguratorBtn').addEventListener('click', () => configuratorModal.classList.add('hidden'));
document.getElementById('cfgCancelBtn').addEventListener('click', () => configuratorModal.classList.add('hidden'));

document.getElementById('cfgSaveBtn').addEventListener('click', async () => {
  const saveBtn = document.getElementById('cfgSaveBtn');
  saveBtn.textContent = 'Применение...';
  saveBtn.disabled = true;

  const checkboxes = document.querySelectorAll('.cfg-device-check');
  const selectedMode = document.querySelector('input[name="apoMode"]:checked')?.value || 'ALL';
  const devicesToApply = [];

  checkboxes.forEach(cb => {
    devicesToApply.push({
      guid: cb.dataset.guid,
      type: cb.dataset.type,
      enabled: cb.checked,
      mode: selectedMode
    });
  });

  try {
    const res = await fetch('/api/apo/apply-batch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ devices: devicesToApply })
    });
    const result = await res.json();
    if (result.status === 'ok') {
      showToast(`Перехват успешно настроен для ${result.applied} устройств! audiodg перезапущен.`);
      configuratorModal.classList.add('hidden');
      await populateDeviceDropdown();
    } else {
      showToast(`Ошибка применения: ${result.errors?.join(', ') || 'Отказано в доступе'}`, true);
    }
  } catch (err) {
    showToast('Сбой применения: запустите редактор от имени Администратора', true);
  } finally {
    saveBtn.textContent = 'Применить перехват';
    saveBtn.disabled = false;
  }
});

async function backupDevice(guid, epType) {
  try {
    const res = await fetch('/api/apo/backup', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ guid, type: epType })
    });
    const data = await res.json();
    if (data.status === 'ok' || data.status === 'created') {
      showToast(`Резервная копия реестра сохранена: ${data.file}`);
    } else {
      showToast('Не удалось создать резервную копию', true);
    }
  } catch (e) {
    showToast('Ошибка выгрузки реестра', true);
  }
}

async function renderConfiguratorDevices() {
  cfgTableBody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding: 20px; color: #64748b;">Загрузка списка аудиоустройств...</td></tr>';

  try {
    if (cachedEndpoints.length === 0) {
      const res = await fetch('/api/devices');
      cachedEndpoints = await res.json();
    }
  } catch (e) {
    console.warn('Could not load devices:', e);
  }

  const targetType = currentCfgTab === 'capture' ? 'Capture' : 'Render';
  const filtered = cachedEndpoints.filter(d => d.endpoint_type === targetType);

  cfgTableBody.innerHTML = '';
  if (filtered.length === 0) {
    cfgTableBody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding: 16px; color: #888;">Устройств не найдено</td></tr>';
    return;
  }

  filtered.forEach(d => {
    const row = document.createElement('tr');
    const isInstalled = d.is_installed;

    row.innerHTML = `
      <td class="cell-center">
        <input type="checkbox" class="cfg-device-check" data-guid="${d.guid}" data-type="${d.endpoint_type}" ${isInstalled ? 'checked' : ''}>
      </td>
      <td>
        <strong style="color: #0f172a">${d.name}</strong>
        <div style="font-size: 10px; color: #888; font-family: monospace;">${d.guid}</div>
      </td>
      <td>${d.connector || 'Audio Endpoint'}</td>
      <td>
        <span style="display: inline-flex; align-items: center; gap: 4px; color: ${isInstalled ? '#107c41' : '#b26b00'}; font-weight: 500;">
          <span style="width:6px; height:6px; border-radius:50%; background-color:${isInstalled ? '#107c41' : '#b26b00'};"></span>
          ${isInstalled ? 'APO активен' : 'Стандартный'}
        </span>
      </td>
      <td class="cell-center">
        <button class="win-btn cfg-backup-btn" data-guid="${d.guid}" data-type="${d.endpoint_type}" style="font-size: 10px; padding: 2px 8px;">Бэкап .reg</button>
      </td>
    `;

    row.querySelector('.cfg-backup-btn').addEventListener('click', () => {
      backupDevice(d.guid, d.endpoint_type);
    });

    cfgTableBody.appendChild(row);
  });
}

async function openConfigurator() {
  currentCfgTab = 'capture';
  document.querySelectorAll('.cfg-tab').forEach(t => {
    t.classList.toggle('active', t.dataset.tab === 'capture');
  });
  cachedEndpoints = [];
  configuratorModal.classList.remove('hidden');
  await renderConfiguratorDevices();
}

// Raw view
document.getElementById('menuRawText').addEventListener('click', () => {
  rawConfigText.value = generateConfig();
  rawConfigModal.classList.remove('hidden');
});
document.getElementById('closeRawModalBtn').addEventListener('click', () => rawConfigModal.classList.add('hidden'));
document.getElementById('cancelRawModalBtn').addEventListener('click', () => rawConfigModal.classList.add('hidden'));
document.getElementById('applyRawConfigBtn').addEventListener('click', () => {
  if (parseConfigIntoState(rawConfigText.value)) {
    renderBlocks();
    updateAll();
    showToast('Текстовая конфигурация успешно применена');
  }
  rawConfigModal.classList.add('hidden');
});

// Toggle bottom panel
togglePanelBtn.addEventListener('click', () => {
  analysisPanel.classList.toggle('collapsed');
  togglePanelBtn.innerHTML = analysisPanel.classList.contains('collapsed') ? '&#9650;' : '&#9660;';
});

// --- New Configuration Dialog (Task 10: Adds new tab) ---
function openNewConfigModal() {
  const filenameInput = document.getElementById('newConfigFilename');
  if (filenameInput) {
    const count = state.tabs.length + 1;
    filenameInput.value = `preset_${count}.txt`;
  }

  const devSel = document.getElementById('newConfigDevice');
  if (devSel) {
    devSel.innerHTML = '<option value="all">Все устройства (All Endpoints - Global)</option>';
    const mainSel = document.getElementById('deviceSelector');
    if (mainSel) {
      for (let i = 0; i < mainSel.options.length; i++) {
        const opt = mainSel.options[i];
        if (opt.value !== 'all') {
          const newOpt = document.createElement('option');
          newOpt.value = opt.value;
          newOpt.textContent = opt.textContent;
          devSel.appendChild(newOpt);
        }
      }
    }
  }

  newConfigModal.classList.remove('hidden');
}

document.getElementById('closeNewConfigBtn').addEventListener('click', () => newConfigModal.classList.add('hidden'));
document.getElementById('btnCancelNewConfig').addEventListener('click', () => newConfigModal.classList.add('hidden'));

document.getElementById('btnConfirmNewConfig').addEventListener('click', () => {
  const filenameInput = document.getElementById('newConfigFilename');
  const chosenFilename = (filenameInput?.value || `preset_${state.tabs.length + 1}.txt`).trim();
  const preset = document.getElementById('newConfigPreset')?.value || 'voice_discord';
  const chosenDevice = document.getElementById('newConfigDevice')?.value || 'all';

  let presetBlocks = [];
  if (preset === 'empty') {
    presetBlocks = [
      { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 }
    ];
  } else if (preset === 'voice_discord') {
    presetBlocks = [
      { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 },
      {
        id: nextId(),
        kind: 'parametric',
        title: 'Параметрические фильтры (Biquad EQ)',
        enabled: true,
        filters: [
          { id: nextId(), type: 'HP', fc: 80, gain: 0.0, q: 0.7071, enabled: true },
          { id: nextId(), type: 'PK', fc: 1200, gain: 3.5, q: 1.4142, enabled: true },
          { id: nextId(), type: 'HS', fc: 8000, gain: 2.5, q: 0.7071, enabled: true },
        ]
      }
    ];
  } else if (preset === 'podcast_warm') {
    presetBlocks = [
      { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 },
      {
        id: nextId(),
        kind: 'parametric',
        title: 'Параметрические фильтры (Biquad EQ)',
        enabled: true,
        filters: [
          { id: nextId(), type: 'HP', fc: 60, gain: 0.0, q: 0.7071, enabled: true },
          { id: nextId(), type: 'PK', fc: 120, gain: 3.5, q: 1.0, enabled: true },
          { id: nextId(), type: 'PK', fc: 2800, gain: 2.0, q: 1.4142, enabled: true },
          { id: nextId(), type: 'HS', fc: 10000, gain: 2.0, q: 0.7071, enabled: true },
        ]
      }
    ];
  } else if (preset === 'noise_cut') {
    presetBlocks = [
      { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 },
      {
        id: nextId(),
        kind: 'parametric',
        title: 'Параметрические фильтры (Biquad EQ)',
        enabled: true,
        filters: [
          { id: nextId(), type: 'HP', fc: 75, gain: 0.0, q: 1.0, enabled: true },
          { id: nextId(), type: 'NO', fc: 50, gain: 0.0, q: 10.0, enabled: true },
          { id: nextId(), type: 'PK', fc: 400, gain: -3.0, q: 2.0, enabled: true },
        ]
      }
    ];
  } else if (preset === 'geq15') {
    presetBlocks = [
      { id: nextId(), kind: 'preamp', title: 'Предусиление (Preamp)', enabled: true, gain: 0.0 },
      {
        id: nextId(),
        kind: 'graphiceq15',
        title: 'Графический эквалайзер (15 полос ISO)',
        enabled: true,
        bands: [
          { f: 25, gain: 0 }, { f: 40, gain: 0 }, { f: 63, gain: 0 },
          { f: 100, gain: 0 }, { f: 160, gain: 0 }, { f: 250, gain: 0 },
          { f: 400, gain: 0 }, { f: 630, gain: 0 }, { f: 1000, gain: 0 },
          { f: 1600, gain: 0 }, { f: 2500, gain: 0 }, { f: 4000, gain: 0 },
          { f: 6300, gain: 0 }, { f: 10000, gain: 0 }, { f: 16000, gain: 0 }
        ]
      }
    ];
  }

  // Task 10: Add new tab instead of overwriting existing
  const newTabId = `tab_${nextId()}`;
  state.tabs.push({
    id: newTabId,
    name: chosenFilename,
    selectedDevice: chosenDevice,
    selectedChannel: 'all',
    blocks: JSON.parse(JSON.stringify(presetBlocks))
  });

  newConfigModal.classList.add('hidden');
  switchTab(newTabId);
  saveConfigToDisk(true);
  showToast(`Создана новая вкладка: ${chosenFilename}`);
});

// --- Save As Modal ---
function openSaveAsModal() {
  const tab = getActiveTab();
  const filenameInp = document.getElementById('saveAsFilename');
  if (filenameInp) filenameInp.value = tab ? tab.name : 'config_backup.txt';
  saveAsModal.classList.remove('hidden');
}

document.getElementById('closeSaveAsBtn').addEventListener('click', () => saveAsModal.classList.add('hidden'));
document.getElementById('btnCancelSaveAs').addEventListener('click', () => saveAsModal.classList.add('hidden'));

document.getElementById('btnConfirmSaveAs').addEventListener('click', () => {
  const filename = document.getElementById('saveAsFilename')?.value || 'config_backup.txt';
  const text = generateConfig();
  const blob = new Blob([text], { type: 'text/plain;charset=utf-8' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  saveAsModal.classList.add('hidden');
  showToast(`Файл ${filename} сохранен и скачан`);
});

// --- Full VST Catalog Modal (98 Plugins) ---
const vstCatalogModal = document.getElementById('vstCatalogModal');
const vstCatalogList = document.getElementById('vstCatalogList');
const vstSearchInput = document.getElementById('vstSearchInput');
const vstCategoryFilter = document.getElementById('vstCategoryFilter');
const closeVstCatalogBtn = document.getElementById('closeVstCatalogBtn');
const cancelVstCatalogBtn = document.getElementById('cancelVstCatalogBtn');

function renderVstCatalog(searchQuery = '', categoryFilter = 'all') {
  if (!vstCatalogList || !Array.isArray(ALL_VST_98_CATALOG)) return;
  const q = searchQuery.trim().toLowerCase();

  const filtered = ALL_VST_98_CATALOG.filter(item => {
    if (categoryFilter !== 'all' && item.cat !== categoryFilter) return false;
    if (!q) return true;
    return (
      (item.title && item.title.toLowerCase().includes(q)) ||
      (item.dll && item.dll.toLowerCase().includes(q)) ||
      (item.desc && item.desc.toLowerCase().includes(q)) ||
      (item.how && item.how.toLowerCase().includes(q)) ||
      (item.catName && item.catName.toLowerCase().includes(q))
    );
  });

  const emptyText = window.t ? window.t('vstModal.emptySearch', null, 'Плагины по запросу не найдены. Попробуйте изменить параметры поиска или категории.') : 'Плагины по запросу не найдены.';
  const purposeLbl = window.t ? window.t('vstModal.purpose', null, 'Назначение:') : 'Назначение:';
  const howLbl = window.t ? window.t('vstModal.principle', null, 'Принцип работы:') : 'Принцип работы:';
  const btnAddTxt = window.t ? window.t('vstModal.btnAdd', null, 'Добавить в цепочку') : 'Добавить в цепочку';

  if (filtered.length === 0) {
    vstCatalogList.innerHTML = `
      <div style="padding: 24px; text-align: center; color: #64748b; font-size: 13px;">
        ${emptyText}
      </div>
    `;
    return;
  }

  let html = '';
  filtered.forEach(p => {
    const meta = getVstLocalizedMeta(p.id, p.dll);
    html += `
      <div class="vst-catalog-card" data-vst-id="${p.id}">
        <div class="vst-card-left">
          <div class="vst-card-title">
            ${meta.title}
            <span class="vst-card-cat">${meta.category}</span>
          </div>
          <span class="vst-card-dll">${p.dll}</span>
          <div class="vst-card-desc"><strong>${purposeLbl}</strong> ${meta.desc}</div>
          <div class="vst-card-how"><strong>${howLbl}</strong> ${meta.howItWorks}</div>
        </div>
        <button class="win-btn primary btn-add-catalog-vst" data-vst-id="${p.id}" data-vst-dll="${p.dll}" data-vst-title="${meta.title}" style="height: 32px; font-weight: 600; padding: 0 14px; white-space: nowrap; flex-shrink: 0;">
          ${btnAddTxt}
        </button>
      </div>
    `;
  });
  vstCatalogList.innerHTML = html;

  vstCatalogList.querySelectorAll('.btn-add-catalog-vst').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const vstId = btn.dataset.vstId;
      const vstDll = btn.dataset.vstDll;
      const vstTitle = btn.dataset.vstTitle;
      const tab = getActiveTab();
      if (!tab) return;

      const newBlock = createVstBlock(vstId, VST_BASE_DIR + vstDll);
      newBlock.title = vstTitle;
      tab.blocks.push(newBlock);

      vstCatalogModal.classList.add('hidden');
      renderBlocks();
      updateAll();
      showToast(`Добавлен VST: ${vstTitle}`);
    });
  });
}

function openVstCatalogModal() {
  if (!vstCatalogModal) return;
  vstCatalogModal.classList.remove('hidden');
  if (vstSearchInput) vstSearchInput.value = '';
  if (vstCategoryFilter) vstCategoryFilter.value = 'all';
  renderVstCatalog('', 'all');
  if (vstSearchInput) vstSearchInput.focus();
}

if (closeVstCatalogBtn) {
  closeVstCatalogBtn.addEventListener('click', () => vstCatalogModal.classList.add('hidden'));
}
if (cancelVstCatalogBtn) {
  cancelVstCatalogBtn.addEventListener('click', () => vstCatalogModal.classList.add('hidden'));
}
if (vstSearchInput) {
  vstSearchInput.addEventListener('input', (e) => {
    renderVstCatalog(e.target.value, vstCategoryFilter ? vstCategoryFilter.value : 'all');
  });
}
if (vstCategoryFilter) {
  vstCategoryFilter.addEventListener('change', (e) => {
    renderVstCatalog(vstSearchInput ? vstSearchInput.value : '', e.target.value);
  });
}

// --- About Modal ---
function openAboutModal() {
  aboutModal.classList.remove('hidden');
}
document.getElementById('closeAboutBtn').addEventListener('click', () => aboutModal.classList.add('hidden'));
document.getElementById('btnOkAbout').addEventListener('click', () => aboutModal.classList.add('hidden'));

// --- Custom VST Plugin Dialog (Replaces prompt()) ---
const customVstModal = document.getElementById('customVstModal');
const customVstPathInput = document.getElementById('customVstPathInput');
const btnBrowseCustomVst = document.getElementById('btnBrowseCustomVst');
const customVstFileInput = document.getElementById('customVstFileInput');
const quickVstFolderSelect = document.getElementById('quickVstFolderSelect');
const btnConfirmCustomVst = document.getElementById('btnConfirmCustomVst');
const btnCancelCustomVst = document.getElementById('btnCancelCustomVst');
const closeCustomVstBtn = document.getElementById('closeCustomVstBtn');

function openCustomVstModal() {
  if (!customVstModal) return;
  if (customVstPathInput) customVstPathInput.value = 'C:\\Program Files\\VSTPlugins\\MyPlugin.dll';
  if (quickVstFolderSelect) quickVstFolderSelect.selectedIndex = 0;
  customVstModal.classList.remove('hidden');
  if (customVstPathInput) customVstPathInput.focus();
}

if (btnBrowseCustomVst && customVstFileInput) {
  btnBrowseCustomVst.addEventListener('click', () => {
    customVstFileInput.click();
  });
  customVstFileInput.addEventListener('change', (e) => {
    if (e.target.files && e.target.files[0]) {
      const f = e.target.files[0];
      if (quickVstFolderSelect && quickVstFolderSelect.value) {
        customVstPathInput.value = quickVstFolderSelect.value + f.name;
      } else {
        customVstPathInput.value = `C:\\Program Files\\VSTPlugins\\${f.name}`;
      }
    }
  });
}

if (quickVstFolderSelect) {
  quickVstFolderSelect.addEventListener('change', (e) => {
    if (e.target.value && customVstPathInput) {
      const cur = customVstPathInput.value.trim();
      const filename = cur.split(/[\\/]/).pop() || 'MyPlugin.dll';
      customVstPathInput.value = e.target.value + filename;
    }
  });
}

if (btnConfirmCustomVst) {
  btnConfirmCustomVst.addEventListener('click', () => {
    const rawPath = customVstPathInput ? customVstPathInput.value.trim() : '';
    if (!rawPath) {
      showToast('Укажите путь к 64-битному .dll плагину', true);
      return;
    }
    const tab = getActiveTab();
    if (tab) {
      tab.blocks.push(createVstBlock('custom', rawPath));
      renderBlocks();
      updateAll();
      showToast('Добавлен сторонний VST плагин');
    }
    customVstModal.classList.add('hidden');
  });
}

if (btnCancelCustomVst) {
  btnCancelCustomVst.addEventListener('click', () => customVstModal.classList.add('hidden'));
}
if (closeCustomVstBtn) {
  closeCustomVstBtn.addEventListener('click', () => customVstModal.classList.add('hidden'));
}

// --- GitHub Auto-Updater Modal (Skill github-desktop-autoupdate) ---
// --- GitHub Auto-Updater Modal (Skill github-desktop-autoupdate) ---
const updateModal = document.getElementById('updateModal');
const closeUpdateBtn = document.getElementById('closeUpdateBtn');
const btnCloseUpdateModal = document.getElementById('btnCloseUpdateModal');
const btnCheckNowUpdate = document.getElementById('btnCheckNowUpdate');
const btnInstallUpdate = document.getElementById('btnInstallUpdate');
const updateStatusText = document.getElementById('updateStatusText');
const updateResultBox = document.getElementById('updateResultBox');
const updateBadgeStatus = document.getElementById('updateBadgeStatus');
const updateNotesText = document.getElementById('updateNotesText');

const updateProgressBox = document.getElementById('updateProgressBox');
const updateProgressLabel = document.getElementById('updateProgressLabel');
const updateProgressPercent = document.getElementById('updateProgressPercent');
const updateProgressBar = document.getElementById('updateProgressBar');
const updateProgressBytes = document.getElementById('updateProgressBytes');
const updateProgressSpeed = document.getElementById('updateProgressSpeed');

let pendingUpdateDownloadUrl = '';
let pendingUpdateAssetSize = 0;
let updatePollTimer = null;
let lastBytesDownloaded = 0;
let lastSpeedTime = Date.now();

function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return '0 B';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
}

function openUpdateModal() {
  if (!updateModal) return;
  updateModal.classList.remove('hidden');
  checkUpdates();
}

async function checkUpdates() {
  if (!btnCheckNowUpdate) return;
  btnCheckNowUpdate.disabled = true;
  btnCheckNowUpdate.textContent = window.t ? window.t('update.checking', null, 'Проверка...') : 'Проверка...';
  if (updateStatusText) updateStatusText.textContent = window.t ? window.t('update.connecting', null, 'Связь с сервером GitHub Releases...') : 'Связь с сервером GitHub Releases...';

  try {
    const res = await fetch('/api/update/check');
    const data = await res.json();

    if (updateCurrentVerText && data.current_version) {
      updateCurrentVerText.textContent = window.t ? window.t('update.currentVer', { ver: data.current_version }, `Текущая версия: v${data.current_version}`) : `Текущая версия: v${data.current_version}`;
    }
    if (updateStatusText) updateStatusText.textContent = data.message || (window.t ? window.t('update.readyToCheck') : 'Проверка завершена');
    if (updateResultBox) updateResultBox.classList.remove('hidden');

    if (data.has_update && data.download_url) {
      pendingUpdateDownloadUrl = data.download_url;
      pendingUpdateAssetSize = data.asset_size || 0;
      if (updateBadgeStatus) {
        updateBadgeStatus.style.background = '#dcfce7';
        updateBadgeStatus.style.color = '#15803d';
        updateBadgeStatus.style.borderColor = '#86efac';
        updateBadgeStatus.textContent = window.t ? window.t('update.available', { ver: data.latest_version, asset: data.asset_name || 'EqualizerFD-Craft.exe' }) : `Доступно обновление: v${data.latest_version} (${data.asset_name || 'EqualizerFD-Craft.exe'})`;
      }
      if (btnInstallUpdate) {
        btnInstallUpdate.classList.remove('hidden');
        btnInstallUpdate.disabled = false;
        btnInstallUpdate.textContent = window.t ? window.t('update.btnDownload', null, 'Скачать и обновить') : 'Скачать и обновить';
      }
    } else {
      pendingUpdateDownloadUrl = '';
      pendingUpdateAssetSize = 0;
      if (updateBadgeStatus) {
        updateBadgeStatus.style.background = '#f1f5f9';
        updateBadgeStatus.style.color = '#334155';
        updateBadgeStatus.style.borderColor = '#cbd5e1';
        updateBadgeStatus.textContent = data.message || (window.t ? window.t('update.upToDate', { ver: data.current_version }) : `У вас установлена последняя версия Equalizer FD CRAFT (v${data.current_version})`);
      }
      if (btnInstallUpdate) btnInstallUpdate.classList.add('hidden');
    }

    if (updateNotesText) {
      updateNotesText.textContent = data.release_notes || (window.t ? window.t('update.notesEmpty') : 'Описание релиза отсутствует.');
    }
  } catch (err) {
    if (updateStatusText) updateStatusText.textContent = window.t ? window.t('update.error', { msg: err.message }) : ('Ошибка проверки обновлений: ' + err.message);
  } finally {
    btnCheckNowUpdate.disabled = false;
    btnCheckNowUpdate.textContent = window.t ? window.t('update.btnCheckNow', null, 'Проверить сейчас') : 'Проверить сейчас';
  }
}

async function pollUpdateProgress() {
  try {
    const res = await fetch('/api/update/progress', { cache: 'no-store' });
    if (!res.ok) {
      waitForServerRestart();
      return;
    }
    const data = await res.json();

    const now = Date.now();
    const dt = (now - lastSpeedTime) / 1000;
    const db = data.downloaded_bytes - lastBytesDownloaded;
    if (dt >= 0.4 && db > 0) {
      const speed = db / dt;
      if (updateProgressSpeed) updateProgressSpeed.textContent = formatBytes(speed) + '/s';
      lastBytesDownloaded = data.downloaded_bytes;
      lastSpeedTime = now;
    }

    const pct = Math.round(data.percent || 0);
    if (updateProgressBar) updateProgressBar.style.width = `${pct}%`;
    if (updateProgressPercent) updateProgressPercent.textContent = `${pct}%`;
    if (updateProgressBytes) {
      updateProgressBytes.textContent = `${formatBytes(data.downloaded_bytes)} / ${formatBytes(data.total_bytes)}`;
    }

    if (data.status === 'downloading') {
      if (updateProgressLabel) updateProgressLabel.textContent = window.t ? window.t('update.progressLabel') : 'Скачивание EqualizerFD-Craft.exe...';
      updatePollTimer = setTimeout(pollUpdateProgress, 120);
    } else if (data.status === 'applying') {
      if (updateProgressBar) {
        updateProgressBar.style.width = '100%';
        updateProgressBar.style.background = '#059669';
      }
      if (updateProgressPercent) updateProgressPercent.textContent = '100%';
      if (updateProgressLabel) updateProgressLabel.textContent = window.t ? window.t('update.applying') : 'Установка обновления и замена файлов...';
      if (updateProgressSpeed) updateProgressSpeed.textContent = window.t ? window.t('update.applyingSpeed', null, 'Применение...') : 'Применение...';
      updatePollTimer = setTimeout(pollUpdateProgress, 120);
    } else if (data.status === 'restarting' || data.status === 'completed') {
      if (updateProgressBar) {
        updateProgressBar.style.width = '100%';
        updateProgressBar.style.background = '#10b981';
      }
      if (updateProgressPercent) updateProgressPercent.textContent = '100%';
      if (updateProgressLabel) updateProgressLabel.textContent = window.t ? window.t('update.restarting') : 'Обновление завершено! Перезапуск студии...';
      if (updateProgressSpeed) updateProgressSpeed.textContent = window.t ? window.t('update.restartingSpeed', null, 'Переподключение...') : 'Переподключение...';
      showToast(window.t ? window.t('toasts.updateRestarting', null, 'Обновление завершено! Перезапуск Equalizer FD CRAFT...') : 'Обновление завершено! Перезапуск Equalizer FD CRAFT...');
      waitForServerRestart();
    } else if (data.status === 'error') {
      if (updateProgressBar) updateProgressBar.style.background = '#dc2626';
      if (updateProgressLabel) updateProgressLabel.textContent = window.t ? window.t('update.error', { msg: data.error_message || 'Не удалось обновить' }) : ('Ошибка: ' + (data.error_message || 'Не удалось обновить'));
      if (btnInstallUpdate) {
        btnInstallUpdate.disabled = false;
        btnInstallUpdate.textContent = window.t ? window.t('update.btnRetry') : 'Повторить';
      }
    } else {
      updatePollTimer = setTimeout(pollUpdateProgress, 150);
    }
  } catch (err) {
    console.log('[Updater] Connection interrupted (server restarting):', err);
    waitForServerRestart();
  }
}

function waitForServerRestart() {
  if (updatePollTimer) clearTimeout(updatePollTimer);
  let attempts = 0;
  const maxAttempts = 40; // up to 20 seconds
  const checkInterval = setInterval(async () => {
    attempts++;
    try {
      const res = await fetch('/api/info', { cache: 'no-store' });
      if (res.ok) {
        clearInterval(checkInterval);
        if (updateProgressLabel) updateProgressLabel.textContent = window.t ? window.t('update.restartedLabel', null, 'Студия перезапущена! Страница обновляется...') : 'Студия перезапущена! Страница обновляется...';
        setTimeout(() => {
          window.location.reload();
        }, 400);
      }
    } catch (_) {
      if (attempts >= maxAttempts) {
        clearInterval(checkInterval);
        if (updateProgressLabel) updateProgressLabel.textContent = window.t ? window.t('update.manualRestart', null, 'Перезапустите приложение вручную.') : 'Перезапустите приложение вручную.';
        if (btnInstallUpdate) {
          btnInstallUpdate.disabled = false;
          btnInstallUpdate.textContent = window.t ? window.t('update.btnDownload', null, 'Скачать и обновить') : 'Скачать и обновить';
        }
      }
    }
  }, 500);
}

if (btnCheckNowUpdate) {
  btnCheckNowUpdate.addEventListener('click', checkUpdates);
}
if (btnInstallUpdate) {
  btnInstallUpdate.addEventListener('click', async () => {
    if (!pendingUpdateDownloadUrl) return;
    btnInstallUpdate.disabled = true;
    btnInstallUpdate.textContent = 'Обновление...';
    if (updateStatusText) updateStatusText.textContent = 'Загрузка и замена файла EqualizerFD-Craft.exe...';

    if (updateProgressBox) updateProgressBox.classList.remove('hidden');
    if (updateProgressBar) {
      updateProgressBar.style.width = '0%';
      updateProgressBar.style.background = '#2563eb';
    }
    if (updateProgressPercent) updateProgressPercent.textContent = '0%';
    if (updateProgressLabel) updateProgressLabel.textContent = 'Подключение к серверу загрузки...';
    if (updateProgressBytes) updateProgressBytes.textContent = `0 B / ${formatBytes(pendingUpdateAssetSize)}`;
    if (updateProgressSpeed) updateProgressSpeed.textContent = 'Подготовка...';

    lastBytesDownloaded = 0;
    lastSpeedTime = Date.now();

    try {
      const res = await fetch('/api/update/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: pendingUpdateDownloadUrl, size: pendingUpdateAssetSize })
      });
      const data = await res.json();
      pollUpdateProgress();
    } catch (err) {
      showToast('Ошибка запуска обновления: ' + err.message, true);
      btnInstallUpdate.disabled = false;
      btnInstallUpdate.textContent = 'Скачать и обновить';
    }
  });
}

if (closeUpdateBtn) closeUpdateBtn.addEventListener('click', () => updateModal.classList.add('hidden'));
if (btnCloseUpdateModal) btnCloseUpdateModal.addEventListener('click', () => updateModal.classList.add('hidden'));

const menuCheckUpdate = document.getElementById('menuCheckUpdate');
const menuHelpUpdate = document.getElementById('menuHelpUpdate');
if (menuCheckUpdate) menuCheckUpdate.addEventListener('click', openUpdateModal);
if (menuHelpUpdate) menuHelpUpdate.addEventListener('click', openUpdateModal);

// --- Microphone Setup Wizard (Tasks 5 & 7: Dynamic Any-Mic Onboarding) ---
async function openWizardModal() {
  wizardModal.classList.remove('hidden');
  const micSel = document.getElementById('wizardMicSelect');
  const renderSel = document.getElementById('wizardRenderSelect');

  if (micSel) micSel.innerHTML = '<option value="">Загрузка микрофонов...</option>';
  if (renderSel) renderSel.innerHTML = '<option value="">Загрузка устройств вывода...</option>';

  try {
    const res = await fetch('/api/devices');
    const devices = await res.json();
    const captures = devices.filter(d => d.endpoint_type === 'Capture');
    const renders = devices.filter(d => d.endpoint_type === 'Render');

    if (micSel) {
      micSel.innerHTML = '';
      captures.forEach(c => {
        const opt = document.createElement('option');
        opt.value = c.guid;
        opt.textContent = `${c.name} (${c.connector || 'Микрофон'})`;
        micSel.appendChild(opt);
      });
      // Select first or remembered
      const saved = localStorage.getItem('resona_selected_device');
      if (saved) micSel.value = saved;
    }

    if (renderSel) {
      renderSel.innerHTML = '';
      renders.forEach(r => {
        const opt = document.createElement('option');
        opt.value = r.guid;
        opt.textContent = `${r.name} (${r.connector || 'Вывод'})`;
        if (r.name.toLowerCase().includes('cable') || r.name.toLowerCase().includes('vb-audio')) {
          opt.selected = true;
        }
        renderSel.appendChild(opt);
      });
    }

    // Check VB-Cable installation status
    const statusBox = document.getElementById('wizardVbCableStatusBox');
    try {
      const vbres = await fetch('/api/vbcable/status');
      const vbdata = await vbres.json();
      if (statusBox) {
        if (vbdata.installed) {
          statusBox.innerHTML = `
            <div style="background:#f0fdf4;border:1px solid #bbf7d0;border-radius:4px;padding:6px 10px;font-size:11px;color:#166534;display:flex;align-items:center;gap:6px;">
              <span style="font-weight:bold;">[+]</span>
              <span>VB-Audio Virtual Cable обнаружен в системе и готов к работе.</span>
            </div>
          `;
        } else {
          statusBox.innerHTML = `
            <div style="background:#fff1f2;border:1px solid #fecdd3;border-radius:4px;padding:8px 10px;font-size:11px;color:#9f1239;">
              <div style="font-weight:600;margin-bottom:4px;">[!] VB-Audio Virtual Cable не обнаружен в вашей системе.</div>
              <div>Для передачи звука в Discord/OBS без задержек необходим виртуальный кабель. Нажмите кнопку для быстрой установки:</div>
              <div style="margin-top:6px;display:flex;align-items:center;gap:8px;">
                <button class="win-btn primary" id="wizardQuickInstallVbCableBtn" style="padding:4px 12px;font-size:11px;background:#0284c7;color:#fff;font-weight:600;">
                  Установить VB-Audio Virtual Cable (1 клик)
                </button>
                <span id="wizardQuickInstallStatus" style="font-size:11px;color:#0369a1;"></span>
              </div>
            </div>
          `;
          document.getElementById('wizardQuickInstallVbCableBtn')?.addEventListener('click', async () => {
            const st = document.getElementById('wizardQuickInstallStatus');
            if (st) st.textContent = 'Запуск установщика...';
            try {
              const r = await fetch('/api/vbcable/install', { method: 'POST' });
              const d = await r.json();
              if (st) st.textContent = d.message || 'Установщик запущен.';
              showToast(d.message || 'Установщик запущен.');
            } catch(e) {
              if (st) st.textContent = 'Ошибка: ' + e.message;
            }
          });
        }
      }
    } catch(e) {}
  } catch (e) {
    if (micSel) micSel.innerHTML = '<option value="all">Все микрофоны (Default)</option>';
  }
}

document.getElementById('closeWizardModalBtn')?.addEventListener('click', () => {
  localStorage.setItem('resona_wizard_done', 'true');
  wizardModal.classList.add('hidden');
});
document.getElementById('wizardSkipBtn')?.addEventListener('click', () => {
  localStorage.setItem('resona_wizard_done', 'true');
  wizardModal.classList.add('hidden');
});

document.getElementById('wizardApplyBtn')?.addEventListener('click', async () => {
  const chosenMic = document.getElementById('wizardMicSelect')?.value || 'all';
  const chosenRender = document.getElementById('wizardRenderSelect')?.value || 'cable';
  const tab = getActiveTab();

  if (chosenMic && tab) {
    tab.selectedDevice = chosenMic;
    localStorage.setItem('resona_selected_device', chosenMic);
    const devSel = document.getElementById('deviceSelector');
    if (devSel) setDeviceSelectorValue(devSel, chosenMic);
  }

  // Start real-time audio bridge with the user's specific microphone!
  try {
    await fetch('/api/stream/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ capture: chosenMic, render: chosenRender })
    });
    showToast('Микрофон подключен к аудиомосту!');
  } catch (e) {}

  localStorage.setItem('resona_wizard_done', 'true');
  wizardModal.classList.add('hidden');
  updateAll();
});

// --- Autostart & Elevation Management (Tasks 4 & 6) ---
async function checkAutostart() {
  try {
    const res = await fetch('/api/autostart');
    if (res.ok) {
      const data = await res.json();
      const chk = document.getElementById('chkAutostart');
      if (chk) chk.textContent = data.enabled ? '[Вкл]' : '[Выкл]';
    }
  } catch (e) {}
}

document.getElementById('menuAutostart')?.addEventListener('click', async () => {
  const chk = document.getElementById('chkAutostart');
  const isCurrentlyOn = chk?.textContent === '[Вкл]';
  const newStatus = !isCurrentlyOn;

  try {
    const res = await fetch('/api/autostart', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: newStatus })
    });
    const data = await res.json();
    if (data.status === 'ok') {
      if (chk) chk.textContent = newStatus ? '[Вкл]' : '[Выкл]';
      showToast(newStatus ? 'Автозагрузка с Windows включена' : 'Автозагрузка с Windows отключена');
    }
  } catch (e) {
    showToast('Не удалось изменить настройки автозагрузки', true);
  }
});

document.getElementById('menuElevate')?.addEventListener('click', async () => {
  if (confirm('Перезапустить Equalizer FD CRAFT от имени Администратора для полного доступа к аудиодрайверам?')) {
    try {
      await fetch('/api/elevate', { method: 'POST' });
      window.close();
    } catch (e) {}
  }
});

// --- Graceful Program Exit (Task 1) ---
async function exitApplication() {
  try {
    await fetch('/api/exit', { method: 'POST' });
  } catch (e) {}
  window.close();
}

document.getElementById('menuExit')?.addEventListener('click', () => {
  saveConfigToDisk(true);
  exitApplication();
});

// --- Toolbar & Menubar Button Actions ---
function openConfigFile() {
  const opener = document.getElementById('fileOpener');
  if (opener) opener.click();
}

function cutBlock() {
  const tab = getActiveTab();
  if (tab && tab.blocks.length > 0) {
    clipboardBlock = tab.blocks.pop();
    renderBlocks();
    updateAll();
    pushHistory();
    showToast("Блок вырезан в буфер");
  }
}

function copyBlock() {
  const tab = getActiveTab();
  if (tab && tab.blocks.length > 0) {
    clipboardBlock = JSON.parse(JSON.stringify(tab.blocks[tab.blocks.length - 1]));
    showToast("Блок скопирован в буфер");
  }
}

function pasteBlock() {
  const tab = getActiveTab();
  if (clipboardBlock && tab) {
    const cloned = JSON.parse(JSON.stringify(clipboardBlock));
    cloned.id = nextId();
    if (cloned.filters) cloned.filters.forEach(f => f.id = nextId());
    tab.blocks.push(cloned);
    renderBlocks();
    updateAll();
    pushHistory();
    showToast("Блок вставлен из буфера");
  }
}

// Hidden file opener change handler
const fileOpener = document.getElementById('fileOpener');
if (fileOpener) {
  fileOpener.addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (evt) => {
      const text = evt.target.result;
      const newTabId = `tab_${nextId()}`;
      state.tabs.push({
        id: newTabId,
        name: file.name,
        selectedDevice: 'all',
        selectedChannel: 'all',
        blocks: []
      });
      switchTab(newTabId);

      if (parseConfigIntoState(text)) {
        renderBlocks();
        updateAll(false);
        pushHistory();
        saveConfigToDisk(true);
        showToast(`Файл ${file.name} открыт в новой вкладке`);
      }
    };
    reader.readAsText(file);
    fileOpener.value = '';
  });
}

// Toolbar buttons
document.getElementById('tbNew')?.addEventListener('click', openNewConfigModal);
document.getElementById('tbOpen')?.addEventListener('click', openConfigFile);
document.getElementById('tbSave')?.addEventListener('click', () => saveConfigToDisk(false));
document.getElementById('tbUndo')?.addEventListener('click', undo);
document.getElementById('tbRedo')?.addEventListener('click', redo);

// Menubar File
document.getElementById('menuNew')?.addEventListener('click', openNewConfigModal);
document.getElementById('menuOpen')?.addEventListener('click', openConfigFile);
document.getElementById('menuSave')?.addEventListener('click', () => saveConfigToDisk(false));
document.getElementById('menuSaveAs')?.addEventListener('click', openSaveAsModal);
document.getElementById('menuImportRew')?.addEventListener('click', openConfigFile);
document.getElementById('menuExportConfig')?.addEventListener('click', openSaveAsModal);

// Menubar Edit
document.getElementById('menuUndo')?.addEventListener('click', undo);
document.getElementById('menuRedo')?.addEventListener('click', redo);
document.getElementById('menuCut')?.addEventListener('click', cutBlock);
document.getElementById('menuCopy')?.addEventListener('click', copyBlock);
document.getElementById('menuPaste')?.addEventListener('click', pasteBlock);

// Menubar View
document.getElementById('menuToggleAnalysis')?.addEventListener('click', () => {
  analysisPanel.classList.toggle('collapsed');
  togglePanelBtn.innerHTML = analysisPanel.classList.contains('collapsed') ? '&#9650;' : '&#9660;';
});

document.getElementById('menuResetZoom')?.addEventListener('click', () => {
  renderAnalysis();
  showToast("Масштаб графика сброшен");
});

// Menubar Settings
document.getElementById('menuWizard')?.addEventListener('click', openWizardModal);
document.getElementById('menuOpenConfigurator')?.addEventListener('click', openConfigurator);
document.getElementById('menuRegistryBackup')?.addEventListener('click', () => {
  const tab = getActiveTab();
  if (tab && tab.selectedDevice && tab.selectedDevice !== 'all') {
    backupDevice(tab.selectedDevice, 'Capture');
  } else {
    openConfigurator();
  }
});

// Menubar Language
function setAppLanguage(lang) {
  const isEn = lang === 'en';
  localStorage.setItem('resona_lang', lang);
  const chkRu = document.getElementById('chkRu');
  const chkEn = document.getElementById('chkEn');
  if (chkRu) chkRu.textContent = isEn ? '' : '[+]';
  if (chkEn) chkEn.textContent = isEn ? '[+]' : '';

  const lblFile = document.getElementById('lblMenuFile');
  const lblEdit = document.getElementById('lblMenuEdit');
  const lblView = document.getElementById('lblMenuView');
  const lblSettings = document.getElementById('lblMenuSettings');
  const lblLang = document.getElementById('lblMenuLanguage');
  const lblHelp = document.getElementById('lblMenuHelp');

  if (lblFile) lblFile.textContent = isEn ? 'File' : 'Файл';
  if (lblEdit) lblEdit.textContent = isEn ? 'Edit' : 'Правка';
  if (lblView) lblView.textContent = isEn ? 'View' : 'Вид';
  if (lblSettings) lblSettings.textContent = isEn ? 'Settings' : 'Настройки';
  if (lblLang) lblLang.textContent = isEn ? 'Language' : 'Язык';
  if (lblHelp) lblHelp.textContent = isEn ? 'Help' : 'Справка';

  showToast(isEn ? 'Interface language: English' : 'Язык интерфейса: Русский');
}

document.getElementById('langRu')?.addEventListener('click', () => setAppLanguage('ru'));
document.getElementById('langEn')?.addEventListener('click', () => setAppLanguage('en'));

// Menubar Help
document.getElementById('menuAbout')?.addEventListener('click', openAboutModal);

// Keyboard Shortcuts
document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
    e.preventDefault();
    if (e.shiftKey) redo();
    else undo();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
    e.preventDefault();
    redo();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
    e.preventDefault();
    saveConfigToDisk(false);
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
    e.preventDefault();
    openNewConfigModal();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {
    e.preventDefault();
    openConfigFile();
  } else if (e.key === 'F8') {
    e.preventDefault();
    analysisPanel.classList.toggle('collapsed');
    togglePanelBtn.innerHTML = analysisPanel.classList.contains('collapsed') ? '&#9650;' : '&#9660;';
  }
});

const deviceSelectorEl = document.getElementById('deviceSelector');
if (deviceSelectorEl) {
  deviceSelectorEl.addEventListener('change', () => {
    const tab = getActiveTab();
    if (tab) {
      tab.selectedDevice = deviceSelectorEl.value;
      localStorage.setItem('resona_selected_device', tab.selectedDevice);
    }
    updateAll();
    pushHistory();
  });
}

const channelSelectorEl = document.getElementById('channelSelector');
if (channelSelectorEl) {
  channelSelectorEl.addEventListener('change', () => {
    const tab = getActiveTab();
    if (tab) {
      tab.selectedChannel = channelSelectorEl.value;
      localStorage.setItem('resona_selected_channel', tab.selectedChannel);
    }
    updateAll();
    pushHistory();
  });
}

async function populateDeviceDropdown() {
  try {
    const res = await fetch('/api/devices');
    if (!res.ok) return;
    const devices = await res.json();
    if (!Array.isArray(devices) || devices.length === 0) return;

    const sel = document.getElementById('deviceSelector');
    if (!sel) return;

    const tab = getActiveTab();
    const savedDev = (tab && tab.selectedDevice) || localStorage.getItem('resona_selected_device') || 'all';
    sel.innerHTML = '';

    const optAll = document.createElement('option');
    optAll.value = 'all';
    optAll.textContent = window.t ? window.t('toolbar.deviceAll', null, 'Все устройства (All Endpoints - Global)') : 'Все устройства (All Endpoints - Global)';
    sel.appendChild(optAll);

    const capGroup = document.createElement('optgroup');
    capGroup.label = window.t ? window.t('toolbar.captureGroup', null, 'Микрофоны (Capture)') : 'Микрофоны (Capture)';

    const renGroup = document.createElement('optgroup');
    renGroup.label = window.t ? window.t('toolbar.renderGroup', null, 'Воспроизведение (Render / Динамики / Наушники)') : 'Воспроизведение (Render / Динамики / Наушники)';

    devices.forEach(d => {
      const opt = document.createElement('option');
      opt.value = d.guid;
      const prefix = d.endpoint_type === 'Capture'
        ? (window.t ? window.t('toolbar.micPrefix', null, 'Микрофон') : 'Микрофон')
        : (window.t ? window.t('toolbar.speakerPrefix', null, 'Динамики') : 'Динамики');
      opt.textContent = `[${prefix}] ${d.name}`;
      if (d.endpoint_type === 'Capture') {
        capGroup.appendChild(opt);
      } else {
        renGroup.appendChild(opt);
      }
    });

    if (capGroup.children.length > 0) sel.appendChild(capGroup);
    if (renGroup.children.length > 0) sel.appendChild(renGroup);

    if (savedDev && setDeviceSelectorValue(sel, savedDev)) {
      // successfully restored
    } else {
      sel.value = 'all';
      if (tab) tab.selectedDevice = 'all';
    }
  } catch (e) {
    console.warn('Could not populate device selector:', e);
  }
}

// App Initialization
async function initApp() {
  if (window.I18N && window.I18N.readyPromise) {
    try { await window.I18N.readyPromise; } catch (_) {}
  }
  renderTabs();
  resizeAnalysisCanvas();

  // Populate devices and autostart asynchronously without blocking UI or config
  populateDeviceDropdown();
  checkAutostart();

  // Load server config into initial tab only if no saved state exists in browser localStorage
  if (!savedTabs || savedTabs.length === 0) {
    try {
      const res = await fetch('/api/config');
      if (res.ok) {
        const data = await res.json();
        if (data && data.config && data.config.trim().length > 0) {
          const hasContent = data.config.split('\n').some(l => {
            const t = l.trim();
            return t && !t.startsWith('#') && (t.startsWith('Preamp:') || t.startsWith('Filter:') || t.startsWith('GraphicEQ:') || t.startsWith('Device:') || t.startsWith('VSTPlugin:') || t.startsWith('BossRC505:') || t.startsWith('Include:'));
          });
          if (hasContent) {
            parseConfigIntoState(data.config);
          }
        }
      }
    } catch (e) {
      console.warn('Could not load server config:', e);
    }
  }

  // Restore remembered channel
  const chEl = document.getElementById('channelSelector');
  const savedCh = localStorage.getItem('resona_selected_channel');
  if (chEl && savedCh) {
    chEl.value = savedCh;
    const tab = getActiveTab();
    if (tab) tab.selectedChannel = savedCh;
  }

  const imCheck = document.getElementById('instantModeCheck');
  if (imCheck) {
    imCheck.checked = state.instantMode;
    imCheck.addEventListener('change', () => {
      state.instantMode = imCheck.checked;
      localStorage.setItem('resona_instant_mode', state.instantMode ? 'true' : 'false');
    });
  }

  renderBlocks();
  renderAnalysis();
  rawConfigText.value = generateConfig();
  pushHistory();
  updateUndoRedoButtons();

  // Task 7: Show First-Run Wizard if not configured yet or requested via URL/hash
  if (localStorage.getItem('resona_wizard_done') !== 'true' || window.location.hash === '#wizard' || window.location.search.includes('wizard')) {
    setTimeout(openWizardModal, 400);
  }

  window.addEventListener('hashchange', () => {
    if (window.location.hash === '#wizard') {
      openWizardModal();
    }
  });
}

initApp();

// --- Virtual Cable Live Stream Poller & Controller ---
async function checkStreamStatus() {
  try {
    const res = await fetch('/api/stream/status');
    if (res.ok) {
      const data = await res.json();
      if (data.open_wizard) {
        openWizardModal();
      }
      const btn = document.getElementById('btnToggleStream');
      const box = document.getElementById('streamBridgeBox');
      if (btn && box) {
        btn.dataset.active = data.active ? 'true' : 'false';
        const isEn = window.I18N && window.I18N.lang === 'en';
        if (data.active) {
          btn.textContent = window.t ? window.t('toolbar.streamLive', null, '[В ЭФИРЕ]') : '[В ЭФИРЕ]';
          btn.style.background = '#16a34a';
          box.style.background = '#dcfce7';
          box.style.borderColor = '#86efac';
        } else {
          btn.textContent = window.t ? window.t('toolbar.streamPaused', null, '[ОСТАНОВЛЕН]') : '[ОСТАНОВЛЕН]';
          btn.style.background = '#64748b';
          box.style.background = '#f1f5f9';
          box.style.borderColor = '#cbd5e1';
        }
      }
    }

    const liveRes = await fetch('/api/live-status');
    if (liveRes.ok) {
      const liveData = await liveRes.json();
      const clipBox = document.getElementById('clipStatusBox');
      const clipTxt = document.getElementById('clipStatusText');
      if (clipBox && clipTxt && liveData.active && liveData.out_peak_db > -90) {
        const outPk = liveData.out_peak_db;
        const isEn = window.I18N && window.I18N.lang === 'en';
        if (outPk > 0.0) {
          clipBox.className = 'status-indicator-clean clip';
          clipTxt.textContent = window.t
            ? window.t('toolbar.clipOverload', { val: outPk.toFixed(1) }, `Клиппинг: +${outPk.toFixed(1)} dB`)
            : `Клиппинг: +${outPk.toFixed(1)} dB`;
        } else {
          clipBox.className = 'status-indicator-clean ok';
          clipTxt.textContent = window.t
            ? window.t('toolbar.liveSignal', { peak: outPk.toFixed(1), gain: liveData.preamp_gain_db.toFixed(1) }, `Сигнал: ${outPk.toFixed(1)} dB (Усиление: +${liveData.preamp_gain_db.toFixed(1)} dB)`)
            : `Сигнал: ${outPk.toFixed(1)} dB (Усиление: +${liveData.preamp_gain_db.toFixed(1)} dB)`;
        }
      }
    }
  } catch (e) {}
}

const btnToggleStream = document.getElementById('btnToggleStream');
if (btnToggleStream) {
  btnToggleStream.addEventListener('click', async () => {
    const isLive = btnToggleStream.dataset.active === 'true' || btnToggleStream.textContent.includes('В ЭФИРЕ') || btnToggleStream.textContent.includes('LIVE');
    btnToggleStream.textContent = '...';
    try {
      if (isLive) {
        await fetch('/api/stream/stop', { method: 'POST' });
        showToast(window.t ? window.t('toasts.streamStopped') : 'WASAPI аудиомост остановлен');
      } else {
        const tab = getActiveTab();
        const cap = tab ? tab.selectedDevice : 'Microphone';
        await fetch('/api/stream/start', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ capture: cap, render: 'cable' })
        });
        showToast(window.t ? window.t('toasts.streamActive') : 'WASAPI аудиомост запущен!');
      }
      setTimeout(checkStreamStatus, 400);
    } catch (e) {
      console.error(e);
    }
  });
}

setInterval(checkStreamStatus, 1000);
checkStreamStatus();

// --- VB-Audio Virtual Cable Dialog Controller ---
const vbCableModal = document.getElementById('vbCableModal');
const menuVbCable = document.getElementById('menuVbCable');
const closeVbCableModalBtn = document.getElementById('closeVbCableModalBtn');
const closeVbCableModalFooterBtn = document.getElementById('closeVbCableModalFooterBtn');
const btnLaunchVbCableInstaller = document.getElementById('btnLaunchVbCableInstaller');
const btnOpenVbCableControlPanel = document.getElementById('btnOpenVbCableControlPanel');
const vbCableStatusBadge = document.getElementById('vbCableStatusBadge');
const vbCableDevicesList = document.getElementById('vbCableDevicesList');
const vbCableInstallMsg = document.getElementById('vbCableInstallMsg');

async function refreshVbCableModalStatus() {
  if (vbCableStatusBadge) {
    vbCableStatusBadge.textContent = 'Проверка...';
    vbCableStatusBadge.style.background = '#e2e8f0';
    vbCableStatusBadge.style.color = '#475569';
  }
  if (vbCableInstallMsg) vbCableInstallMsg.textContent = '';

  try {
    const res = await fetch('/api/vbcable/status');
    const data = await res.json();
    if (vbCableStatusBadge) {
      if (data.installed) {
        vbCableStatusBadge.textContent = 'Установлен и готов';
        vbCableStatusBadge.style.background = '#dcfce7';
        vbCableStatusBadge.style.color = '#15803d';
      } else {
        vbCableStatusBadge.textContent = 'Не установлен в системе';
        vbCableStatusBadge.style.background = '#fee2e2';
        vbCableStatusBadge.style.color = '#b91c1c';
      }
    }
    if (vbCableDevicesList) {
      if (data.devices && data.devices.length > 0) {
        vbCableDevicesList.innerHTML = '<strong>Обнаруженные устройства:</strong><br>' + data.devices.map(d => `• ${d}`).join('<br>');
      } else {
        vbCableDevicesList.innerHTML = '<em>Устройства VB-Audio не найдены в диспетчере звука Windows. Нажмите кнопку ниже для установки.</em>';
      }
    }
  } catch (e) {
    if (vbCableStatusBadge) {
      vbCableStatusBadge.textContent = 'Ошибка проверки';
      vbCableStatusBadge.style.background = '#fee2e2';
      vbCableStatusBadge.style.color = '#b91c1c';
    }
  }
}

function openVbCableModal() {
  if (vbCableModal) {
    vbCableModal.classList.remove('hidden');
    refreshVbCableModalStatus();
  }
}

function closeVbCableModal() {
  if (vbCableModal) {
    vbCableModal.classList.add('hidden');
  }
}

if (menuVbCable) {
  menuVbCable.addEventListener('click', () => {
    openVbCableModal();
  });
}

if (closeVbCableModalBtn) closeVbCableModalBtn.addEventListener('click', closeVbCableModal);
if (closeVbCableModalFooterBtn) closeVbCableModalFooterBtn.addEventListener('click', closeVbCableModal);

if (btnLaunchVbCableInstaller) {
  btnLaunchVbCableInstaller.addEventListener('click', async () => {
    if (vbCableInstallMsg) vbCableInstallMsg.textContent = 'Запуск установщика драйвера от имени Администратора...';
    try {
      const res = await fetch('/api/vbcable/install', { method: 'POST' });
      const data = await res.json();
      if (vbCableInstallMsg) vbCableInstallMsg.textContent = data.message || 'Установщик запущен.';
      showToast(data.message || 'Установщик запущен.');
      setTimeout(refreshVbCableModalStatus, 5000);
    } catch (e) {
      if (vbCableInstallMsg) vbCableInstallMsg.textContent = 'Ошибка вызова установщика: ' + e.message;
    }
  });
}

if (btnOpenVbCableControlPanel) {
  btnOpenVbCableControlPanel.addEventListener('click', async () => {
    try {
      const res = await fetch('/api/vbcable/control-panel', { method: 'POST' });
      const data = await res.json();
      showToast(data.message || 'Панель управления открыта.');
    } catch (e) {
      showToast('Ошибка запуска: ' + e.message);
    }
  });
}

// Global Exports and Localization Listeners
window.showToast = showToast;

window.addEventListener('languageChanged', (e) => {
  renderTabs();
  renderBlocks();
  renderAnalysis();
  populateDeviceDropdown();
  checkStreamStatus();
});

// Native Desktop Feel: Eliminate all browser manifestations
// 1. Disable default browser context menu (Inspect Element, Back, Reload, Print)
window.addEventListener('contextmenu', (e) => {
  const tag = (e.target && e.target.tagName) || '';
  if (tag !== 'INPUT' && tag !== 'TEXTAREA') {
    e.preventDefault();
  }
});

// 3. Block browser developer keys, print, search, and page reload shortcuts
window.addEventListener('keydown', (e) => {
  // Block F12 (DevTools), Ctrl+Shift+I, Ctrl+Shift+J, Ctrl+Shift+C, Ctrl+U (Page Source)
  if (e.key === 'F12' || (e.ctrlKey && e.shiftKey && ['I', 'J', 'C', 'i', 'j', 'c'].includes(e.key)) || (e.ctrlKey && (e.key === 'u' || e.key === 'U'))) {
    e.preventDefault();
    return;
  }
  // Block Ctrl+P (Print), Ctrl+F (Find in page), F3, Ctrl+H (History), Ctrl+J (Downloads), Ctrl+O (Browser Open)
  if ((e.ctrlKey && ['p', 'P', 'f', 'F', 'h', 'H', 'j', 'J'].includes(e.key)) || e.key === 'F3') {
    e.preventDefault();
    return;
  }
  // Block F5 and Ctrl+R (Browser Reload)
  if (e.key === 'F5' || (e.ctrlKey && (e.key === 'r' || e.key === 'R'))) {
    e.preventDefault();
    return;
  }
});

// 4. Disable accidental dragging of images, cards, and UI links
window.addEventListener('dragstart', (e) => {
  const tag = (e.target && e.target.tagName) || '';
  if (tag !== 'INPUT' && tag !== 'TEXTAREA') {
    e.preventDefault();
  }
});
window.addEventListener('drop', (e) => e.preventDefault());


