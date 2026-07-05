import { useEffect, useMemo, useRef, useState } from 'react';

type Locale = 'en' | 'zh';

type ToneEvent =
  | {
      freq_hz: number;
      ms: number;
    }
  | {
      rest_ms: number;
    };

type Candidate = {
  id: string;
  label: 'recommended' | 'alternate';
  events: ToneEvent[];
};

type Tone = {
  id: string;
  group: string;
  kind: 'one-shot' | 'continuous' | 'interval';
  tags: string[];
  loopGapMs?: number;
  candidates: Candidate[];
};

type LocalizedTone = {
  title: string;
  policy: string;
};

type LocalizedCandidate = {
  intent: string;
};

const toneData = {
  audio: {
    waveform: 'square' as OscillatorType,
    volume: 0.34,
    fadeMs: 3,
  },
  tones: [
    {
      id: 'boot',
      group: 'system',
      kind: 'one-shot',
      tags: ['system', 'startup'],
      candidates: [
        {
          id: 'boot-rise',
          label: 'recommended',
          events: [
            { freq_hz: 660, ms: 70 },
            { rest_ms: 35 },
            { freq_hz: 880, ms: 85 },
            { rest_ms: 35 },
            { freq_hz: 1320, ms: 120 },
          ],
        },
        {
          id: 'boot-soft',
          label: 'alternate',
          events: [{ freq_hz: 587, ms: 90 }, { rest_ms: 45 }, { freq_hz: 988, ms: 150 }],
        },
      ],
    },
    {
      id: 'operation-ok',
      group: 'button',
      kind: 'one-shot',
      tags: ['button', 'accepted'],
      candidates: [
        {
          id: 'op-ok-tick',
          label: 'recommended',
          events: [{ freq_hz: 1047, ms: 45 }, { rest_ms: 25 }, { freq_hz: 1319, ms: 60 }],
        },
        {
          id: 'op-ok-single',
          label: 'alternate',
          events: [{ freq_hz: 1175, ms: 70 }],
        },
      ],
    },
    {
      id: 'operation-denied',
      group: 'button',
      kind: 'one-shot',
      tags: ['button', 'denied'],
      candidates: [
        {
          id: 'op-denied-low',
          label: 'recommended',
          events: [{ freq_hz: 440, ms: 80 }, { rest_ms: 35 }, { freq_hz: 330, ms: 110 }],
        },
        {
          id: 'op-denied-buzz',
          label: 'alternate',
          events: [{ freq_hz: 260, ms: 55 }, { rest_ms: 35 }, { freq_hz: 260, ms: 55 }],
        },
      ],
    },
    {
      id: 'channel-power-on',
      group: 'channel-power',
      kind: 'one-shot',
      tags: ['channel', 'power-on'],
      candidates: [
        {
          id: 'ch-on-rise',
          label: 'recommended',
          events: [{ freq_hz: 784, ms: 60 }, { rest_ms: 30 }, { freq_hz: 1175, ms: 90 }],
        },
        {
          id: 'ch-on-pop',
          label: 'alternate',
          events: [{ freq_hz: 1480, ms: 80 }],
        },
      ],
    },
    {
      id: 'channel-power-off',
      group: 'channel-power',
      kind: 'one-shot',
      tags: ['channel', 'power-off'],
      candidates: [
        {
          id: 'ch-off-fall',
          label: 'recommended',
          events: [{ freq_hz: 1175, ms: 55 }, { rest_ms: 35 }, { freq_hz: 784, ms: 95 }],
        },
        {
          id: 'ch-off-low',
          label: 'alternate',
          events: [{ freq_hz: 392, ms: 95 }],
        },
      ],
    },
    {
      id: 'alarm-over-temp',
      group: 'continuous-alarm',
      kind: 'continuous',
      loopGapMs: 280,
      tags: ['alarm', 'temperature'],
      candidates: [
        {
          id: 'temp-alarm-triplet',
          label: 'recommended',
          events: [
            { freq_hz: 1320, ms: 70 },
            { rest_ms: 50 },
            { freq_hz: 1320, ms: 70 },
            { rest_ms: 50 },
            { freq_hz: 988, ms: 120 },
          ],
        },
        {
          id: 'temp-alarm-saw',
          label: 'alternate',
          events: [
            { freq_hz: 1568, ms: 80 },
            { rest_ms: 45 },
            { freq_hz: 1047, ms: 80 },
            { rest_ms: 45 },
            { freq_hz: 1568, ms: 80 },
          ],
        },
      ],
    },
    {
      id: 'alarm-input-over-power',
      group: 'continuous-alarm',
      kind: 'continuous',
      loopGapMs: 300,
      tags: ['alarm', 'input-power'],
      candidates: [
        {
          id: 'input-power-steady',
          label: 'recommended',
          events: [{ freq_hz: 740, ms: 120 }, { rest_ms: 70 }, { freq_hz: 740, ms: 120 }],
        },
        {
          id: 'input-power-descend',
          label: 'alternate',
          events: [
            { freq_hz: 880, ms: 80 },
            { rest_ms: 45 },
            { freq_hz: 660, ms: 80 },
            { rest_ms: 45 },
            { freq_hz: 494, ms: 110 },
          ],
        },
      ],
    },
    {
      id: 'alarm-channel-short',
      group: 'continuous-alarm',
      kind: 'continuous',
      loopGapMs: 220,
      tags: ['alarm', 'short-circuit'],
      candidates: [
        {
          id: 'short-urgent',
          label: 'recommended',
          events: [
            { freq_hz: 220, ms: 70 },
            { rest_ms: 45 },
            { freq_hz: 220, ms: 70 },
            { rest_ms: 45 },
            { freq_hz: 330, ms: 70 },
          ],
        },
        {
          id: 'short-hard',
          label: 'alternate',
          events: [{ freq_hz: 196, ms: 95 }, { rest_ms: 40 }, { freq_hz: 196, ms: 95 }],
        },
      ],
    },
    {
      id: 'alarm-channel-over-5a',
      group: 'interval-alarm',
      kind: 'interval',
      loopGapMs: 1800,
      tags: ['alarm', 'current-5a'],
      candidates: [
        {
          id: 'over5a-ping',
          label: 'recommended',
          events: [{ freq_hz: 988, ms: 80 }, { rest_ms: 80 }, { freq_hz: 988, ms: 80 }],
        },
        {
          id: 'over5a-mark',
          label: 'alternate',
          events: [{ freq_hz: 1047, ms: 100 }],
        },
      ],
    },
    {
      id: 'hint-current-3a',
      group: 'channel-hint',
      kind: 'one-shot',
      tags: ['hint', 'current-3a'],
      candidates: [
        {
          id: 'current3a-nudge',
          label: 'recommended',
          events: [{ freq_hz: 880, ms: 55 }, { rest_ms: 40 }, { freq_hz: 988, ms: 55 }],
        },
        {
          id: 'current3a-soft',
          label: 'alternate',
          events: [{ freq_hz: 932, ms: 75 }],
        },
      ],
    },
    {
      id: 'hint-current-5a',
      group: 'channel-hint',
      kind: 'one-shot',
      tags: ['hint', 'current-5a'],
      candidates: [
        {
          id: 'current5a-firm',
          label: 'recommended',
          events: [{ freq_hz: 988, ms: 60 }, { rest_ms: 45 }, { freq_hz: 784, ms: 80 }],
        },
        {
          id: 'current5a-high',
          label: 'alternate',
          events: [{ freq_hz: 1319, ms: 50 }, { rest_ms: 45 }, { freq_hz: 1319, ms: 70 }],
        },
      ],
    },
    {
      id: 'hint-insert',
      group: 'channel-hint',
      kind: 'one-shot',
      tags: ['hint', 'insert'],
      candidates: [
        {
          id: 'insert-bright',
          label: 'recommended',
          events: [{ freq_hz: 784, ms: 45 }, { rest_ms: 25 }, { freq_hz: 1047, ms: 70 }],
        },
        {
          id: 'insert-click',
          label: 'alternate',
          events: [{ freq_hz: 1200, ms: 55 }],
        },
      ],
    },
    {
      id: 'hint-remove',
      group: 'channel-hint',
      kind: 'one-shot',
      tags: ['hint', 'remove'],
      candidates: [
        {
          id: 'remove-fall',
          label: 'recommended',
          events: [{ freq_hz: 1047, ms: 45 }, { rest_ms: 30 }, { freq_hz: 698, ms: 80 }],
        },
        {
          id: 'remove-soft',
          label: 'alternate',
          events: [{ freq_hz: 622, ms: 85 }],
        },
      ],
    },
  ] satisfies Tone[],
};

const copyBase = {
  en: {
    all: 'All',
    alternate: 'Alternate',
    duration: 'duration',
    fade: 'fade',
    filter: 'Filter group',
    gap: 'gap',
    groupLabel: 'Group',
    loop: 'Loop',
    loopPlaying: 'Stop loop',
    play: 'Play',
    playAll: 'Play recommended set',
    playing: 'Playing:',
    ready: 'Click any play button to unlock browser audio output.',
    recommended: 'Recommended',
    rest: 'rest',
    stop: 'Stop',
    stopped: 'Stopped.',
    summaryAudio: 'browser square wave',
    summaryCandidates: 'two candidates each',
    summaryHardware: 'GPIO7 PWM buzzer',
    summaryTones: 'audible events',
    title: 'Buzzer audio workbench',
    subtitle:
      'Audition the passive-buzzer cues used by firmware. The page uses browser Web Audio square waves and keeps the source timing visible for review.',
    waveform: 'waveform',
  },
  zh: {
    all: '全部',
    alternate: '备选',
    duration: '时长',
    fade: '淡入淡出',
    filter: '显示分组',
    gap: '间隔',
    groupLabel: '分组',
    loop: '循环',
    loopPlaying: '停止循环',
    play: '播放',
    playAll: '播放全部推荐',
    playing: '正在播放：',
    ready: '点击任意播放按钮后，浏览器会解锁音频输出。',
    recommended: '推荐',
    rest: '静音',
    stop: '停止',
    stopped: '已停止。',
    summaryAudio: '浏览器方波合成',
    summaryCandidates: '每项两个候选',
    summaryHardware: 'GPIO7 PWM 蜂鸣器',
    summaryTones: '有声音事件',
    title: '蜂鸣器音效工作台',
    subtitle:
      '试听固件使用的无源蜂鸣器提示音。页面使用浏览器 Web Audio 方波合成，并保留每个候选的毫秒级时序。',
    waveform: '波形',
  },
} satisfies Record<Locale, Record<string, string>>;

const groups = {
  en: {
    button: 'Front-panel input',
    'channel-hint': 'Port hints',
    'channel-power': 'Port power',
    'continuous-alarm': 'Continuous alarms',
    'interval-alarm': 'Interval alarms',
    system: 'System',
  },
  zh: {
    button: '按键',
    'channel-hint': '通道提示',
    'channel-power': '通道电源',
    'continuous-alarm': '持续告警',
    'interval-alarm': '间隔告警',
    system: '系统',
  },
} satisfies Record<Locale, Record<string, string>>;

const tones = {
  en: {
    'alarm-channel-over-5a': {
      policy:
        'Loops with a long gap when a port remains above 5 A; lower priority than fault alarms.',
      title: 'Port current above 5 A',
    },
    'alarm-channel-short': {
      policy: 'Loops with a short gap when a port has very low voltage and high current.',
      title: 'Port short-circuit alarm',
    },
    'alarm-input-over-power': {
      policy: 'Loops with a short gap until input power returns to a safe envelope.',
      title: 'Input over-power alarm',
    },
    'alarm-over-temp': {
      policy: 'Loops with a short gap until the over-temperature condition clears.',
      title: 'Over-temperature alarm',
    },
    boot: {
      policy: 'Played once after self-check reaches a runnable state.',
      title: 'Boot tone',
    },
    'channel-power-off': {
      policy: 'Played when a port output changes from enabled to disabled.',
      title: 'Port power-off tone',
    },
    'channel-power-on': {
      policy: 'Played when a port output changes from disabled to enabled.',
      title: 'Port power-on tone',
    },
    'hint-current-3a': {
      policy: 'Played once when a port first crosses the 3 A current threshold.',
      title: 'Current reached 3 A',
    },
    'hint-current-5a': {
      policy:
        'Played once when a port first crosses 5 A; sustained current then becomes an interval alarm.',
      title: 'Current reached 5 A',
    },
    'hint-insert': {
      policy: 'Played once when a port voltage rises past 3.3 V.',
      title: 'Insert hint',
    },
    'hint-remove': {
      policy: 'Played once when a port voltage falls below 3 V.',
      title: 'Remove hint',
    },
    'operation-denied': {
      policy: 'Played when a button action is rejected by safety policy.',
      title: 'Operation denied tone',
    },
    'operation-ok': {
      policy: 'Played when a button press actually performs an operation.',
      title: 'Operation cue',
    },
  },
  zh: {
    'alarm-channel-over-5a': {
      policy: '长间隔循环，提醒但低于持续故障告警优先级。',
      title: '通道电流超过 5 A',
    },
    'alarm-channel-short': {
      policy: '电压很低且电流很大时短间隔循环。',
      title: '通道短路告警',
    },
    'alarm-input-over-power': {
      policy: '短间隔循环，直到输入功率恢复安全。',
      title: '输入过功率告警',
    },
    'alarm-over-temp': {
      policy: '短间隔循环，直到过温解除。',
      title: '过温告警',
    },
    boot: {
      policy: '上电后自检进入可运行阶段时播放一次。',
      title: '开机音',
    },
    'channel-power-off': {
      policy: '某通道输出从打开变为关闭时播放。',
      title: '通道断电音',
    },
    'channel-power-on': {
      policy: '某通道输出从关闭变为打开时播放。',
      title: '通道上电音',
    },
    'hint-current-3a': {
      policy: '通道电流首次跨过 3 A 阈值时播放一次。',
      title: '电流达到 3 A',
    },
    'hint-current-5a': {
      policy: '通道电流首次跨过 5 A 阈值时播放一次；之后若持续超过 5 A 进入间隔告警。',
      title: '电流达到 5 A',
    },
    'hint-insert': {
      policy: '通道电压突破 3.3 V 时播放一次。',
      title: '插入提示音',
    },
    'hint-remove': {
      policy: '通道电压跌破 3 V 时播放一次。',
      title: '拔出提示音',
    },
    'operation-denied': {
      policy: '按键触发但操作被安全策略拒绝时播放。',
      title: '操作拒绝音',
    },
    'operation-ok': {
      policy: '按键触发且确实产生操作时播放。',
      title: '操作提示音',
    },
  },
} satisfies Record<Locale, Record<string, LocalizedTone>>;

const candidates = {
  en: {
    'boot-rise': 'Short upward phrase that says the device is awake.',
    'boot-soft': 'A softer two-note boot acknowledgement.',
    'ch-off-fall': 'Short falling phrase paired with the power-on tone.',
    'ch-off-low': 'Lower single note for disconnect.',
    'ch-on-pop': 'Single high note with more action.',
    'ch-on-rise': 'Short rising phrase, distinct from the boot tone.',
    'current3a-nudge': 'Light notification, not alarm-like.',
    'current3a-soft': 'Single mid-high note.',
    'current5a-firm': 'Clearer than 3 A, but not a fault alarm.',
    'current5a-high': 'More noticeable high double-tap.',
    'input-power-descend': 'Short descending phrase with a more severe feel.',
    'input-power-steady': 'Two equal pulses to communicate total input pressure.',
    'insert-bright': 'Bright single rise.',
    'insert-click': 'Very short attach cue.',
    'op-denied-buzz': 'Short double pulse, more forceful.',
    'op-denied-low': 'Falling double note, clear but not harsh.',
    'op-ok-single': 'Single crisp short note.',
    'op-ok-tick': 'Quick confirmation that does not compete with alarms.',
    'over5a-mark': 'Single brief marker, least intrusive.',
    'over5a-ping': 'Two notes per cycle with a long reminder gap.',
    'remove-fall': 'Short falling phrase paired with insert.',
    'remove-soft': 'Lower-intensity offline cue.',
    'short-hard': 'Harder repeated low note.',
    'short-urgent': 'Fast low double pulse, highest urgency.',
    'temp-alarm-saw': 'High-low sweep to indicate thermal protection.',
    'temp-alarm-triplet': 'Three mid-high notes, persistent but distinct from short-circuit.',
  },
  zh: {
    'boot-rise': '短促上扬，表示设备已醒来。',
    'boot-soft': '更温和的双音开机反馈。',
    'ch-off-fall': '短下行，和上电音成对。',
    'ch-off-low': '低频单音，表示断开。',
    'ch-on-pop': '单个高音，动作感更强。',
    'ch-on-rise': '短上扬，区别于开机音。',
    'current3a-nudge': '轻提示，不像告警。',
    'current3a-soft': '单个中高音。',
    'current5a-firm': '比 3 A 更明确，但不等同故障。',
    'current5a-high': '更醒目的高音双击。',
    'input-power-descend': '短下行，偏严重。',
    'input-power-steady': '两个等长强提示，表达总输入压力。',
    'insert-bright': '明亮单上扬。',
    'insert-click': '极短接入提示。',
    'op-denied-buzz': '短促双脉冲，偏强硬。',
    'op-denied-low': '下行双音，明确但不刺耳。',
    'op-ok-single': '单个清脆短音。',
    'op-ok-tick': '轻快确认，不抢告警优先级。',
    'over5a-mark': '单次短促标记，最不打扰。',
    'over5a-ping': '每轮两声，长间隔提醒。',
    'remove-fall': '短下行，和插入提示成对。',
    'remove-soft': '低强度离线提示。',
    'short-hard': '更硬的重复低音。',
    'short-urgent': '快速低频双脉冲，最高紧急度。',
    'temp-alarm-saw': '高低摆动，提示热保护。',
    'temp-alarm-triplet': '三连中高音，持续但不混同短路。',
  },
} satisfies Record<Locale, Record<string, string>>;

function sequenceDurationMs(events: ToneEvent[]) {
  return events.reduce((total, event) => total + ('rest_ms' in event ? event.rest_ms : event.ms), 0);
}

function eventLabel(event: ToneEvent, copy: Record<string, string>) {
  return 'rest_ms' in event
    ? `${copy.rest} ${event.rest_ms}ms`
    : `${event.freq_hz}Hz / ${event.ms}ms`;
}

function tagTone(tag: string) {
  if (tag === 'alarm' || tag === 'short-circuit') return 'danger';
  if (tag.includes('5a') || tag === 'temperature' || tag === 'input-power') return 'warn';
  if (tag === 'accepted' || tag === 'startup' || tag === 'insert') return 'ok';
  return '';
}

export function BuzzerAudioPreview({ locale }: { locale: Locale }) {
  const copy = copyBase[locale];
  const audioContextRef = useRef<AudioContext | null>(null);
  const loopTimerRef = useRef<number | null>(null);
  const sequenceTimersRef = useRef<number[]>([]);
  const activeNodesRef = useRef<OscillatorNode[]>([]);
  const [groupFilter, setGroupFilter] = useState('all');
  const [loopKey, setLoopKey] = useState<string | null>(null);
  const [status, setStatus] = useState(copy.ready);

  const groupOptions = useMemo(
    () => [...new Set(toneData.tones.map((tone) => tone.group))],
    [],
  );

  const visibleTones = useMemo(
    () =>
      groupFilter === 'all'
        ? toneData.tones
        : toneData.tones.filter((tone) => tone.group === groupFilter),
    [groupFilter],
  );

  function ensureAudio() {
    if (!audioContextRef.current) {
      const AudioContextClass =
        window.AudioContext ||
        (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AudioContextClass) {
        setStatus('Web Audio API is not available in this browser.');
        return Promise.reject(new Error('AudioContext unavailable'));
      }
      audioContextRef.current = new AudioContextClass();
    }
    if (audioContextRef.current.state === 'suspended') {
      return audioContextRef.current.resume();
    }
    return Promise.resolve();
  }

  function stopAll(nextStatus = copy.stopped, updateUi = true) {
    if (loopTimerRef.current) {
      window.clearTimeout(loopTimerRef.current);
      loopTimerRef.current = null;
    }

    sequenceTimersRef.current.forEach((timer) => window.clearTimeout(timer));
    sequenceTimersRef.current = [];

    activeNodesRef.current.forEach((node) => {
      try {
        node.stop(0);
      } catch (_) {
        // Already stopped.
      }
    });
    activeNodesRef.current = [];
    if (updateUi) {
      setLoopKey(null);
      setStatus(nextStatus);
    }
  }

  useEffect(() => {
    return () => {
      stopAll(copy.stopped, false);
      const context = audioContextRef.current;
      audioContextRef.current = null;
      if (context && context.state !== 'closed') {
        void context.close().catch(() => undefined);
      }
    };
  }, []);

  function playEvents(events: ToneEvent[], label: string) {
    const context = audioContextRef.current;
    if (!context) return;

    let cursor = context.currentTime + 0.015;
    const fade = toneData.audio.fadeMs / 1000;

    events.forEach((event) => {
      if ('rest_ms' in event) {
        cursor += event.rest_ms / 1000;
        return;
      }

      const duration = event.ms / 1000;
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = toneData.audio.waveform;
      oscillator.frequency.setValueAtTime(event.freq_hz, cursor);

      const edge = Math.min(fade, duration / 3);
      const attackEnd = cursor + edge;
      const releaseStart = cursor + Math.max(0, duration - edge);
      gain.gain.setValueAtTime(0.0001, cursor);
      gain.gain.linearRampToValueAtTime(toneData.audio.volume, attackEnd);
      gain.gain.setValueAtTime(toneData.audio.volume, releaseStart);
      gain.gain.linearRampToValueAtTime(0.0001, cursor + duration);

      oscillator.connect(gain).connect(context.destination);
      oscillator.start(cursor);
      oscillator.stop(cursor + duration + 0.01);
      oscillator.addEventListener('ended', () => {
        oscillator.disconnect();
        gain.disconnect();
      });
      activeNodesRef.current.push(oscillator);
      cursor += duration;
    });

    setStatus(`${copy.playing} ${label}`);
  }

  async function playCandidate(tone: Tone, candidate: Candidate) {
    stopAll(copy.ready);
    await ensureAudio();
    playEvents(candidate.events, `${tones[locale][tone.id].title} / ${copy[candidate.label]}`);
  }

  async function toggleLoop(tone: Tone, candidate: Candidate) {
    const key = `${tone.id}:${candidate.id}`;
    if (loopKey === key) {
      stopAll();
      return;
    }

    stopAll(copy.ready);
    await ensureAudio();
    setLoopKey(key);

    const run = () => {
      playEvents(candidate.events, `${tones[locale][tone.id].title} / ${copy[candidate.label]}`);
      loopTimerRef.current = window.setTimeout(
        run,
        sequenceDurationMs(candidate.events) + (tone.loopGapMs || 600),
      );
    };
    run();
  }

  async function playRecommendedSet() {
    stopAll(copy.ready);
    await ensureAudio();

    let delay = 0;
    toneData.tones.forEach((tone) => {
      const candidate = tone.candidates[0];
      const timer = window.setTimeout(() => {
        playEvents(candidate.events, `${tones[locale][tone.id].title} / ${copy.recommended}`);
      }, delay);
      sequenceTimersRef.current.push(timer);
      delay += sequenceDurationMs(candidate.events) + 420;
    });
  }

  return (
    <section className="isohub-audio-preview" aria-label={copy.title}>
      <div className="isohub-audio-console">
        <div className="isohub-audio-console__copy">
          <span className="isohub-audio-kicker">{copy.summaryHardware}</span>
          <h2>{copy.title}</h2>
          <p>{copy.subtitle}</p>
        </div>
        <div className="isohub-audio-toolbar">
          <button className="isohub-audio-button primary" type="button" onClick={playRecommendedSet}>
            {copy.playAll}
          </button>
          <button className="isohub-audio-button secondary" type="button" onClick={() => stopAll()}>
            {copy.stop}
          </button>
        </div>
      </div>

      <div className="isohub-audio-summary" aria-label={copy.title}>
        <div>
          <strong>{toneData.tones.length}</strong>
          <span>{copy.summaryTones}</span>
        </div>
        <div>
          <strong>2</strong>
          <span>{copy.summaryCandidates}</span>
        </div>
        <div>
          <strong>GPIO7</strong>
          <span>PWM</span>
        </div>
        <div>
          <strong>{toneData.audio.waveform}</strong>
          <span>
            {copy.waveform}, {toneData.audio.fadeMs}ms {copy.fade}
          </span>
        </div>
      </div>

      <div className="isohub-audio-controls">
        <label>
          {copy.filter}
          <select value={groupFilter} onChange={(event) => setGroupFilter(event.target.value)}>
            <option value="all">{copy.all}</option>
            {groupOptions.map((group) => (
              <option key={group} value={group}>
                {groups[locale][group]}
              </option>
            ))}
          </select>
        </label>
        <div className="isohub-audio-status" role="status" aria-live="polite">
          {status}
        </div>
      </div>

      <div className="isohub-audio-list">
        {visibleTones.map((tone) => {
          const toneText = tones[locale][tone.id];
          return (
            <article className="isohub-tone-card" data-kind={tone.kind} key={tone.id}>
              <div className="isohub-tone-card__head">
                <div>
                  <span className="isohub-tone-card__group">{groups[locale][tone.group]}</span>
                  <h3>{toneText.title}</h3>
                </div>
                <p>{toneText.policy}</p>
                <div className="isohub-tone-tags">
                  {tone.tags.map((tag) => (
                    <span className={tagTone(tag)} key={tag}>
                      {tag}
                    </span>
                  ))}
                </div>
              </div>

              <div className="isohub-candidate-list">
                {tone.candidates.map((candidate) => {
                  const total = sequenceDurationMs(candidate.events);
                  const currentLoopKey = `${tone.id}:${candidate.id}`;
                  const isLooping = loopKey === currentLoopKey;
                  return (
                    <section className="isohub-candidate" key={candidate.id}>
                      <div className="isohub-candidate__title">
                        <strong>{copy[candidate.label]}</strong>
                        <span>{candidates[locale][candidate.id]}</span>
                      </div>
                      <div className="isohub-timeline" aria-label={`${toneText.title} ${copy.duration}`}>
                        <div className="isohub-timeline__bar" aria-hidden="true">
                          {candidate.events.map((event, index) => {
                            const width =
                              ('rest_ms' in event ? event.rest_ms : event.ms) / Math.max(total, 1);
                            return (
                              <span
                                className={'rest_ms' in event ? 'rest' : undefined}
                                key={`${candidate.id}-${index}`}
                                style={{ flexGrow: Math.max(width, 0.05) }}
                                title={eventLabel(event, copy)}
                              />
                            );
                          })}
                        </div>
                        <div className="isohub-timeline__meta">
                          <span>{total}ms</span>
                          {tone.kind !== 'one-shot' && tone.loopGapMs ? (
                            <span>
                              {copy.gap} {tone.loopGapMs}ms
                            </span>
                          ) : null}
                          <span>
                            {candidate.events
                              .filter((event): event is Extract<ToneEvent, { freq_hz: number }> =>
                                'freq_hz' in event,
                              )
                              .map((event) => `${event.freq_hz}Hz/${event.ms}ms`)
                              .join(', ')}
                          </span>
                        </div>
                      </div>
                      <div className="isohub-candidate__actions">
                        <button
                          className="isohub-icon-button"
                          type="button"
                          onClick={() => playCandidate(tone, candidate)}
                          aria-label={`${copy.play} ${toneText.title} / ${copy[candidate.label]}`}
                          title={copy.play}
                        >
                          <span className="isohub-play-icon" aria-hidden="true" />
                        </button>
                        {tone.kind !== 'one-shot' ? (
                          <button
                            className={`isohub-icon-button${isLooping ? ' is-active' : ''}`}
                            type="button"
                            onClick={() => toggleLoop(tone, candidate)}
                            aria-label={`${isLooping ? copy.loopPlaying : copy.loop} ${toneText.title} / ${
                              copy[candidate.label]
                            }`}
                            title={isLooping ? copy.loopPlaying : copy.loop}
                          >
                            <span
                              className={isLooping ? 'isohub-stop-icon' : 'isohub-loop-icon'}
                              aria-hidden="true"
                            />
                          </button>
                        ) : null}
                      </div>
                    </section>
                  );
                })}
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}
