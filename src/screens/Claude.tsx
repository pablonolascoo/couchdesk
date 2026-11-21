import { useEffect, useRef, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import {
  PaperPlaneRight, Sparkle, Wrench, Plus, ChatsCircle, PlugsConnected, PuzzlePiece, PictureInPicture,
  Microphone, SpeakerHigh, SpeakerSlash, GearSix, ArrowClockwise, Desktop, WarningCircle,
} from '@phosphor-icons/react'
import { Page } from '../components/Page'
import { call } from '../native'
import { useT } from '../i18n'
import {
  suggestions, useClaudeChat, useClaudeStatus, useVoice, useHoldToTalk, useSpeakSetting,
  currentConversation, setCurrentConversation, type Msg,
} from '../data/claude'
import './Claude.css'

export function Bubbles({ msgs, busy, empty }: { msgs: Msg[]; busy: boolean; empty: string }) {
  const t = useT()
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => { ref.current?.scrollTo({ top: ref.current.scrollHeight, behavior: 'smooth' }) }, [msgs, busy])
  return (
    <div className="chat" ref={ref}>
      {msgs.length === 0 && <p className="hero-meta chat-empty"><Sparkle size={18} weight="fill" /> {empty}</p>}
      {msgs.map((m, i) => (
        <div key={i} className={`bubble ${m.role}`}>
          {m.role === 'tool' ? <><Wrench size={14} weight="light" /> {t('claude.using', { tool: m.text })}</> : m.text}
        </div>
      ))}
      {busy && <div className="bubble claude typing">{t('claude.thinking')}</div>}
    </div>
  )
}

/** Caja de pregunta con micrófono: mantener pulsado (ratón) o pulsar A para empezar y otra vez para enviar */
export function AskBox({ chat, voice, idPrefix = '' }: {
  chat: ReturnType<typeof useClaudeChat>
  voice: ReturnType<typeof useVoice>
  idPrefix?: string
}) {
  const t = useT()
  const [text, setText] = useState('')
  const fromPointer = useRef(false)
  const send = (q: string) => { chat.ask(q); setText('') }
  return (
    <form className="ask" onSubmit={(e) => { e.preventDefault(); send(text) }}>
      <button
        type="button"
        className={`mic ${voice.listening ? 'on' : ''}`}
        data-nav={`${idPrefix}mic`}
        data-nav-default=""
        aria-label={voice.listening ? t('claude.mic.release') : t('claude.mic.hold')}
        disabled={chat.busy}
        onPointerDown={(e) => { if (e.button === 0) { fromPointer.current = true; voice.start() } }}
        onPointerUp={() => voice.stop()}
        onPointerLeave={() => { if (fromPointer.current) voice.stop() }}
        onClick={() => {
          if (fromPointer.current) { fromPointer.current = false; return }
          if (voice.listening) voice.stop(); else voice.start()
        }}
      >
        <Microphone size={22} weight={voice.listening ? 'fill' : 'light'} />
      </button>
      <input
        className="ask-input"
        data-nav={`${idPrefix}ask-input`}
        value={voice.listening ? voice.partial : text}
        readOnly={voice.listening}
        onChange={(e) => setText(e.target.value)}
        placeholder={voice.listening ? t('claude.ask.listening') : t('claude.ask.placeholder')}
      />
      <button className="btn-primary send" data-nav={`${idPrefix}send`} type="submit" disabled={chat.busy || !text.trim()}>
        <PaperPlaneRight size={18} weight="fill" /> {t('claude.ask.send')}
      </button>
    </form>
  )
}

export function VoiceError({ voice, idPrefix = '' }: { voice: ReturnType<typeof useVoice>; idPrefix?: string }) {
  const t = useT()
  if (!voice.error) return null
  return (
    <div className="notice error voice-error" role="alert">
      <WarningCircle size={20} weight="light" />
      <span>{voice.error}</span>
      <button className="btn-ghost small" data-nav={`${idPrefix}voice-settings`} onClick={voice.openSettings}>
        <GearSix size={18} weight="light" /> {t('claude.voice.openSettings')}
      </button>
    </div>
  )
}

/** Claude Code falta o no tiene sesión: qué pasa y cómo arreglarlo */
function Setup({ installed, onRetry }: { installed: boolean; onRetry: () => void }) {
  const t = useT()
  const navigate = useNavigate()
  // "Escribe {cmd} y pulsa Intro": el comando va en un <kbd> en medio de la frase
  const typeStep = (cmd: string, key: string) => {
    const [before, after = ''] = t(key).split('{cmd}')
    return <>{before}<kbd className="kb">{cmd}</kbd>{after}</>
  }
  return (
    <div className="empty-state claude-setup">
      <span className="empty-icon"><Sparkle size={26} weight="light" /></span>
      <h2>{installed ? t('claude.setup.loginTitle') : t('claude.setup.installTitle')}</h2>
      {installed ? (
        <ol>
          <li>{t('claude.setup.step.desktop')}</li>
          <li>{t('claude.setup.step.terminal')}</li>
          <li>{typeStep('claude', 'claude.setup.step.type')}</li>
          <li>{t('claude.setup.step.login')}</li>
        </ol>
      ) : (
        <ol>
          <li>{t('claude.setup.step.desktop')}</li>
          <li>{t('claude.setup.step.terminal')}</li>
          <li>{typeStep('irm https://claude.ai/install.ps1 | iex', 'claude.setup.step.type')}</li>
          <li>{typeStep('claude', 'claude.setup.step.afterInstall')}</li>
        </ol>
      )}
      <p>{t('claude.setup.account')}</p>
      <div className="hero-actions">
        <button className="btn-primary" data-nav="setup-retry" data-nav-default="" onClick={onRetry}><ArrowClockwise size={18} weight="light" /> {t('claude.setup.retry')}</button>
        <button className="btn-ghost" data-nav="setup-desktop" onClick={() => navigate('/escritorio')}><Desktop size={18} weight="light" /> {t('claude.setup.goDesktop')}</button>
      </div>
    </div>
  )
}

export function Claude() {
  const t = useT()
  const navigate = useNavigate()
  const [params, setParams] = useSearchParams()
  const [statusKey, setStatusKey] = useState(0)
  const status = useClaudeStatus(statusKey)
  const want = params.get('c')
  const chat = useClaudeChat((sid) => { if (params.get('c') !== sid) setParams({ c: sid }, { replace: true }) })
  const idRef = useRef(chat.id)
  idRef.current = chat.id
  const voice = useVoice((heard) => { if (heard.trim()) chat.ask(heard); else voice.setError(t('claude.voice.notHeard')) })
  useHoldToTalk(voice.start, voice.stop)
  const [speakOn, toggleSpeak] = useSpeakSetting()

  // La dirección manda: /claude?c=<id> abre esa conversación; /claude sigue la última
  useEffect(() => {
    if (!want) {
      const cur = currentConversation()
      if (cur) { setParams({ c: cur }, { replace: true }); return }
    }
    if ((want ?? null) !== idRef.current) chat.open(want)
  }, [want]) // eslint-disable-line react-hooks/exhaustive-deps

  const fresh = () => { setCurrentConversation(null); chat.open(null); setParams({}, { replace: true }) }

  if (status && (!status.installed || status.loggedIn === false)) {
    return <Page title="Claude"><Setup installed={status.installed} onRetry={() => setStatusKey((k) => k + 1)} /></Page>
  }

  return (
    <Page title="Claude" hints={[['A', t('common.select')], ['X', t('claude.hint.talk')], ['B', t('common.back')]]}>
      <nav className="claude-bar" aria-label={t('claude.bar.label')}>
        <button className="c-chip" data-nav="c-new" onClick={fresh}><Plus size={18} weight="light" /> {t('claude.new')}</button>
        <button className="c-chip" data-nav="c-list" onClick={() => navigate('/claude/conversaciones')}><ChatsCircle size={18} weight="light" /> {t('claude.conversations')}</button>
        <button className="c-chip" data-nav="c-conn" onClick={() => navigate('/claude/conectores')}><PlugsConnected size={18} weight="light" /> {t('claude.connectors')}</button>
        <button className="c-chip" data-nav="c-ext" onClick={() => navigate('/claude/extensiones')}><PuzzlePiece size={18} weight="light" /> {t('claude.extensions')}</button>
        <button className="c-chip" data-nav="c-float" onClick={() => call('assistant_show').catch(() => {})}><PictureInPicture size={18} weight="light" /> {t('claude.openFloating')}</button>
      </nav>

      <Bubbles msgs={chat.msgs} busy={chat.busy} empty={t('claude.empty')} />

      {chat.msgs.length === 0 && (
        <div className="chips">
          {suggestions().map((s, i) => (
            <button key={s} className="option tool" data-nav={`sug-${i}`} onClick={() => chat.ask(s)} disabled={chat.busy}>{s}</button>
          ))}
        </div>
      )}

      <AskBox chat={chat} voice={voice} />
      <VoiceError voice={voice} />

      <div className="claude-foot">
        <button className="c-chip" data-nav="c-speak" aria-pressed={speakOn} onClick={toggleSpeak}>
          {speakOn ? <SpeakerHigh size={18} weight="light" /> : <SpeakerSlash size={18} weight="light" />}
          {speakOn ? t('claude.speak.on') : t('claude.speak.off')}
        </button>
        <p className="claude-help">
          {(() => {
            // "…: {keys} o mantén {pad} en el mando.": las teclas van en <kbd>
            const [a, rest = ''] = t('claude.floatHelp').split('{keys}')
            const [b, c = ''] = rest.split('{pad}')
            return <>{a}<kbd className="kb">{t('claude.floatHelp.keys')}</kbd>{b}<kbd className="kb">View + Y</kbd>{c}</>
          })()}
        </p>
      </div>
    </Page>
  )
}
